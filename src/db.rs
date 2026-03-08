use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use std::sync::Mutex;

use crate::models::ClimateReading;

const RETENTION_DAYS: i64 = 180;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the SQLite database and run migrations.
    pub fn open(path: &str) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS climate_readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                temperature REAL NOT NULL,
                humidity INTEGER NOT NULL,
                recorded_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_device_time
                ON climate_readings(device_id, recorded_at);
            ",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Insert a new climate reading.
    pub fn insert_reading(
        &self,
        device_id: &str,
        temperature: f64,
        humidity: i32,
        recorded_at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO climate_readings (device_id, temperature, humidity, recorded_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![device_id, temperature, humidity, recorded_at.to_rfc3339()],
        )?;
        Ok(())
    }

    /// Query readings within a time range for a device.
    pub fn query_readings(
        &self,
        device_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ClimateReading>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT temperature, humidity, recorded_at
             FROM climate_readings
             WHERE device_id = ?1 AND recorded_at >= ?2 AND recorded_at <= ?3
             ORDER BY recorded_at ASC",
        )?;

        let readings = stmt
            .query_map(
                params![device_id, start.to_rfc3339(), end.to_rfc3339()],
                |row| {
                    let recorded_at_str: String = row.get(2)?;
                    let recorded_at = DateTime::parse_from_rfc3339(&recorded_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    Ok(ClimateReading {
                        temperature: row.get(0)?,
                        humidity: row.get(1)?,
                        recorded_at,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(readings)
    }

    /// Delete readings older than the retention period.
    pub fn cleanup_old_readings(&self) -> Result<usize, DbError> {
        let conn = self.conn.lock().unwrap();
        let cutoff = Utc::now() - chrono::Duration::days(RETENTION_DAYS);
        let deleted = conn.execute(
            "DELETE FROM climate_readings WHERE recorded_at < ?1",
            params![cutoff.to_rfc3339()],
        )?;
        Ok(deleted)
    }
}
