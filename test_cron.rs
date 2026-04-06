fn main() {
    let expr = "0 * * * * *";
    let _cron: croner::Cron = expr.parse().unwrap();
}
