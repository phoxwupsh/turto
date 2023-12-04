use std::sync::OnceLock;
use reqwest::Client;

pub mod guild;
pub mod json;
pub mod misc;
pub mod play;
pub mod template;
pub mod ytdl;

pub fn get_http_client() -> Client {
    static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
    HTTP_CLIENT.get_or_init(Client::new).clone()
}