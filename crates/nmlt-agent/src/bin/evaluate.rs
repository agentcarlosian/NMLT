use nmlt_agent::evaluate_held_out_suite;

fn main() {
    match evaluate_held_out_suite() {
        Ok(report) => println!("{}", report.to_json()),
        Err(error) => {
            eprintln!("nmlt-agent evaluation failed: {error}");
            std::process::exit(1);
        }
    }
}
