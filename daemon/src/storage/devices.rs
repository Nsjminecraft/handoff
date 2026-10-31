use anyhow::Result;
use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub addr: String,
    pub public_key: Vec<u8>,
    pub paired_at: i64,
    pub last_seen: Option<i64>,
}

pub struct DeviceStore;

impl DeviceStore {
    pub fn save(conn: &Connection, device: &Device) -> Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO devices (id, name, addr, public_key, paired_at, last_seen) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                device.id,
                device.name,
                device.addr,
                device.public_key,
                device.paired_at,
                device.last_seen,
            ],
        )?;
        Ok(())
    }

    pub fn get(conn: &Connection, id: &str) -> Result<Option<Device>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, addr, public_key, paired_at, last_seen FROM devices WHERE id = ?1",
        )?;
        let result = stmt
            .query_row(rusqlite::params![id], |row| {
                Ok(Device {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    addr: row.get(2)?,
                    public_key: row.get(3)?,
                    paired_at: row.get(4)?,
                    last_seen: row.get(5)?,
                })
            })
            .ok();
        Ok(result)
    }

    pub fn list(conn: &Connection) -> Result<Vec<Device>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, addr, public_key, paired_at, last_seen FROM devices ORDER BY last_seen DESC",
        )?;
        let devices = stmt
            .query_map([], |row| {
                Ok(Device {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    addr: row.get(2)?,
                    public_key: row.get(3)?,
                    paired_at: row.get(4)?,
                    last_seen: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(devices)
    }

    pub fn delete(conn: &Connection, id: &str) -> Result<()> {
        conn.execute("DELETE FROM devices WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    pub fn update_last_seen(conn: &Connection, id: &str) -> Result<()> {
        conn.execute(
            "UPDATE devices SET last_seen = ?1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().timestamp(), id],
        )?;
        Ok(())
    }
}
