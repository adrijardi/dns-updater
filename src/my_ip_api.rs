
pub async fn get_my_ip() -> Result<String, String> {

    reqwest::get("https://checkip.amazonaws.com")
    .await
    .map_err(|e| format!("Failed to retrieve ip address: {}", e).to_string())?
    .text()
        .await
        .map(|r| r.trim().to_string())
        .map_err(|e| format!("Failed to extract ip address: {}", e).to_string())
}
