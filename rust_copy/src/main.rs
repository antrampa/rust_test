use std::env;
use std::fs;

fn main() {
    let args : Vec<String> = env::args().collect();
    let argc = args.len();
    if argc < 3 {
        println!("Give source and destination files: rust_copy <copy from> <copy to>");
        return;
    }


    println!("argc: {}", argc);
    println!("args: {:?}", args);

    let source_file = &args[1];
    let destination_file = &args[2];
    
    println!("source_file: {}", source_file);
    println!("destination_file: {}", destination_file);

    let content = fs::read_to_string(source_file)
        .expect("Cannot read the file");

    println!("File content: {}", content);
    

    println!("Copy Done!");
}
