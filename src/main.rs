use LKH::*;

fn main() {
    println!("hello lkh");
    println!("hello lkh");
    println!("hello lkh");
    println!("hello lkh");
    unsafe {
        for _ in 0..10 {
            println!("{}", GetTime());
        }
    }
}
