const BANNER_STR: &'static str = include_str!("../banner.txt");

pub fn print_banner() {
  println!("{}", BANNER_STR);
}