extern crate edid_rs;

fn main() {
    println!("{:?}", edid_rs::parse(&mut std::io::stdin()));
}