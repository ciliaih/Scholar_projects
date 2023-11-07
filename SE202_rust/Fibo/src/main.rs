/*fn fibo(n: u32) -> u32 {
    // Recursivite
    let _i = if n < 2 { return n; } else { return fibo(n-1)+fibo(n-2); };
}*/
use clap::Parser;
#[derive(Parser, Default, Debug)]
#[command(author = "Cilia Ihadadene", about = "Compute Fibonacci suite values")]

struct Args {
    /// The maximal number to print the fibo value of
    #[clap(name = "VALUE")]
    value: u32,

    /// Print intermediate values
    #[clap(short, long)]
    verbose: bool,

    /// The minimum number to compute
    #[clap(short, long, default_value = "0")]
    min: u32,
}

fn fibo(n: u32) -> Option<u32> {
    // Iterativite
    let mut fibo_0: u32 = 0;
    let mut fibo_1: u32 = 1;
    let mut somme = 0;

    match n {
        0 => Some(fibo_0),
        1 => Some(fibo_1),

        _ => {
            for _ in 2..=n {
                somme = fibo_0.checked_add(fibo_1)?;
                fibo_0 = fibo_1;
                fibo_1 = somme;
            }
            Some(somme)
        }
    }
}

fn main() {
    let args = Args::parse();

    for i in args.min..=args.value {
        if let Some(x) = fibo(i) {
            println!("fibo({i}) = {x}");
        }
    }
}
