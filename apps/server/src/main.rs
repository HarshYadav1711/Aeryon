use aeryon_config::version;
use aeryon_events::name as events_name;

fn main() {
    println!("aeryon-server {}", version());
    println!("events subsystem: {}", events_name());
}
