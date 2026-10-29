// ------------------------------------------------------------
// Module: app
// Purpose: The running host. It owns the table lock, the data
//          directory, and a short debounce before each save.
// ------------------------------------------------------------

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Mutex};

use crate::assets::{load_catalog, Catalog};
use crate::config::Config;
use crate::house::House;
use crate::session::Table;
use crate::storage;
use crate::systems::{load_systems, SystemCatalog};

pub struct App {
    pub table: Mutex<Table>,
    pub house: Mutex<House>,
    pub data_dir: PathBuf,
    pub static_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub systems_dir: PathBuf,
    pub catalog: Mutex<Catalog>,
    pub systems: Mutex<SystemCatalog>,
    save_tx: mpsc::UnboundedSender<()>,
}

impl App {
    pub fn boot(config: Config) -> Arc<Self> {
        let data_dir = PathBuf::from(&config.data_dir);
        let assets_dir = PathBuf::from(&config.assets_dir);
        let systems_dir = PathBuf::from(&config.systems_dir);
        let _ = std::fs::create_dir_all(data_dir.join("maps"));
        let _ = std::fs::create_dir_all(data_dir.join("tokens"));
        let _ = std::fs::create_dir_all(data_dir.join("tables"));
        let _ = std::fs::create_dir_all(&systems_dir);
        let catalog = load_catalog(&assets_dir).unwrap_or_else(|err| {
            tracing::error!("asset packs were not loaded: {err}");
            Catalog::default()
        });
        let systems = load_systems(&systems_dir);

        let mut table = Table::new();
        storage::load_into(&mut table, &data_dir);
        table.passphrase = config.table_passphrase.clone();
        table.gm_key = config.gm_key.clone();
        table.sync_fog_size();

        let (house, fresh) = House::load(&data_dir, &config.table_passphrase, &table.campaign.name);
        if fresh {
            if let Some(id) = house.open_id() {
                let path = data_dir.join("tables").join(format!("{id}.json"));
                if let Err(err) = storage::write_to(&table, &path) {
                    tracing::warn!("could not keep the first table: {err}");
                }
            }
        } else if let Some(id) = house.open_id() {
            let path = data_dir.join("tables").join(format!("{id}.json"));
            if path.is_file() {
                storage::load_file(&mut table, &path);
                table.passphrase = config.table_passphrase.clone();
                table.gm_key = config.gm_key.clone();
            } else if let Err(err) = storage::write_to(&table, &path) {
                tracing::warn!("could not keep the open table: {err}");
            }
        }

        table.catalog_revision = catalog.revision;
        let (save_tx, save_rx) = mpsc::unbounded_channel();
        let app = Arc::new(Self {
            table: Mutex::new(table),
            house: Mutex::new(house),
            data_dir,
            static_dir: PathBuf::from(config.static_dir),
            assets_dir,
            systems_dir,
            catalog: Mutex::new(catalog),
            systems: Mutex::new(systems),
            save_tx,
        });
        spawn_saver(app.clone(), save_rx);
        app
    }

    pub fn request_save(&self) {
        let _ = self.save_tx.send(());
    }

    pub async fn save_now(&self) {
        let open_id = self.house.lock().await.open_id();
        let table = self.table.lock().await;
        if let Err(err) = storage::write(&table, &self.data_dir) {
            tracing::warn!("could not save the table: {err}");
        }
        if let Some(id) = open_id {
            let path = self.data_dir.join("tables").join(format!("{id}.json"));
            if let Err(err) = storage::write_to(&table, &path) {
                tracing::warn!("could not save the open table: {err}");
            }
        }
    }
}

fn spawn_saver(app: Arc<App>, mut save_rx: mpsc::UnboundedReceiver<()>) {
    tokio::spawn(async move {
        while save_rx.recv().await.is_some() {
            tokio::time::sleep(Duration::from_millis(300)).await;
            while save_rx.try_recv().is_ok() {}
            app.save_now().await;
        }
    });
}
