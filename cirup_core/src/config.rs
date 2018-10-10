
#[cfg(test)]
use toml;

#[derive(Serialize,Deserialize)]
struct Config {
    repository: Repository,
}

#[derive(Serialize,Deserialize)]
struct Repository {
    path: String,
    resources: Resources,
}

#[derive(Serialize,Deserialize)]
struct Resources {
    dir: String,
    filter: String,
}

#[test]
fn config_write() {
    let config = Config {
        repository: Repository {
            path: "xxx".to_string(),
            resources: Resources {
                dir: "yyy".to_string(),
                filter: "*.json".to_string(),
            }
        },
    };

    let toml = toml::to_string(&config).unwrap();
    println!("{}", toml);
}
