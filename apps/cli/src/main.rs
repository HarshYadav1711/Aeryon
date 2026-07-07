use aeryon_config::version;
use aeryon_plugins::name as plugins_name;

fn main() {
    println!("aeryon-cli {}", version());
    println!("plugins subsystem: {}", plugins_name());
}
