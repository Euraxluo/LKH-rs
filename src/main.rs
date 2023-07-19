use LKH::*;

fn main() {
    println!("hello lkh");
    unsafe {
        hello();
        println!("{}", add(1, 2));
        bye();
        bye();
        bye();
        bye();
        for _ in 0..2 {
            println!("{}", multiply(2.5, 3.0));
            println!("{}", GetTime());
        }
    }
}
