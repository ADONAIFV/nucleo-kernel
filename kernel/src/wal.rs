//! Write-Ahead Log para persistencia y recuperación.

use anyhow::Result;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub struct Wal {
    log_path: PathBuf,
    checkpoint_path: PathBuf,
}

impl Wal {
    pub fn new(dir: &Path) -> Result<Self> {
        let log_path = dir.join("nucleo.wal");
        let checkpoint_path = dir.join("nucleo.checkpoint");
        if !log_path.exists() {
            File::create(&log_path)?;
        }
        Ok(Self {
            log_path,
            checkpoint_path,
        })
    }

    pub fn append(&self, entry: &str) -> Result<()> {
        let mut file = OpenOptions::new().append(true).open(&self.log_path)?;
        let crc = crc32fast::hash(entry.as_bytes());
        writeln!(file, "{}|{}", crc, entry)?;
        file.sync_all()?;
        Ok(())
    }

    pub fn replay(&self) -> Result<()> {
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() != 2 {
                eprintln!("⚠️ Línea mal formada en WAL ({}): {}", line_num + 1, line);
                continue;
            }

            let crc_str = parts[0];
            let entry = parts[1];
            let crc_actual = crc32fast::hash(entry.as_bytes());
            let crc_expected: u32 = crc_str.parse().unwrap_or(0);

            if crc_actual != crc_expected {
                eprintln!(
                    "❌ Corrupción en WAL línea {}: CRC no coincide",
                    line_num + 1
                );
                continue;
            }

            entries.push(entry.to_string());
        }

        if entries.is_empty() {
            println!("📭 WAL vacío.");
        } else {
            println!("📜 Replay WAL ({} entradas):", entries.len());
            for (i, e) in entries.iter().enumerate() {
                println!("  {}. {}", i + 1, e);
            }
        }

        Ok(())
    }

    pub fn checkpoint(&self) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let content = format!("CHECKPOINT {}", timestamp);
        fs::write(&self.checkpoint_path, content)?;
        Ok(())
    }

    pub fn shutdown(&self) -> Result<()> {
        self.append("SHUTDOWN")?;
        self.checkpoint()?;
        Ok(())
    }
}
