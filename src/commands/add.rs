use crate::configuration::Settings;
use crate::database::Database;

async fn _dip_is_clone(config: &Settings, _value: &str) -> bool {
    let _db = Database::connect(config).await;
    todo!()
}

pub async fn add(_config: &Settings, value: &str) {
    let _dir = std::env::current_dir().expect("Failed to find current directory");
    let _conn = println!("we are adding, {:?}", value);
}
