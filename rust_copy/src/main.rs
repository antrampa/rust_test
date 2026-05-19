use std::env;

fn main() {
    let args : Vec<String> = env::args().collect();
    let argc = args.len();
    if argc < 3 {
        println!("Give source and destination files: rust_copy <copy from> <copy to>");
        return;
    }
    println!("argc: {}", argc);
    println!("args: {:?}", args);
    println!("Done!");
}
