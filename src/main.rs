use reqwest::blocking::Client;

fn main() -> Result<(), String> {
    let Some(token) = ghet_rektstension::token_for_host("github.com") else {
        return Err("oops".to_string());
    };

    // Make an API request to /user
    //

    let client = Client::new();
    let req = client
        .get("https://api.github.com/user")
        .header("User-Agent", "ghet-rektstension")
        .header("Authorization", format!("token {}", token.value));

    let resp = client
        .execute(req.build().map_err(|err| "Error".to_string())?)
        .map_err(|err| "Error".to_string())?
        .text()
        .map_err(|err| "Error".to_string())?;

    println!("Ok: {resp}");

    // Print the resulting JSON to stdout

    Ok(())
}
