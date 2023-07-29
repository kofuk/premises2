pub mod v1;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Config {
    #[serde(rename = "v1")]
    V1(v1::Config),
}
