use sts_core::flow4b::RunState4b;
fn main() {
    let mut run = RunState4b::new(424242);
    run.choose_node(3, 0).ok();
    run.relics.push("Vajra");
    let json = run.save_json();
    println!("{}", json);
}
