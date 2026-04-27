pub fn show_current_user() {
    let config = match crate::config::AppConfig::load() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    match crate::auth::current_user(&config) {
        Ok(user) => {
            println!("Current user: {}", user.username);
            println!("Role: {:?}", user.role);
            println!("Auth enabled: {}", config.auth.enabled);
        }
        Err(error) => eprintln!("{error}"),
    }
}
