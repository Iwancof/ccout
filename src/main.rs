use ccout::cap_stdout;

unsafe extern "C" {
    fn puts(s: *const u8) -> i32;
}

fn main() {
    let mut threads = Vec::new();
    for tid in 0..5 {
        let thread = std::thread::spawn(move || {
            for i in 0..10 {
                let r = cap_stdout(|| unsafe {
                    puts(format!("Hello, world! {}\0", i).as_ptr());

                    // sleep random time
                    std::thread::sleep(std::time::Duration::from_millis(
                        rand::random::<u64>() % 10,
                    ));

                    puts(b"goodbye\0".as_ptr());
                })
                .unwrap();

                println!("cap_stdout thread {}: {}", tid, i);

                assert_eq!(r, format!("Hello, world! {}\ngoodbye\n", i));
            }
        });

        threads.push(thread);
    }

    for thread in threads {
        thread.join().unwrap();
    }

    let mut threads = Vec::new();

    for tid in 0..5 {
        let thread = std::thread::spawn(move || {
            for i in 0..10 {
                unsafe {
                    puts(format!("Hello from outside of cap_stdout! {}\0", i).as_ptr());

                    // sleep random time
                    std::thread::sleep(std::time::Duration::from_millis(
                        rand::random::<u64>() % 10,
                    ));

                    puts(b"goodbye~~~\0".as_ptr());
                }

                println!("outside cap_stdout thread {}: {}", tid, i);
            }
        });

        threads.push(thread);
    }

    for thread in threads {
        thread.join().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main() {
        let r = cap_stdout(|| unsafe {
            puts(b"Hello, world!\0".as_ptr());
        })
        .unwrap();

        assert_eq!(r, "Hello, world!\n");
    }
}
