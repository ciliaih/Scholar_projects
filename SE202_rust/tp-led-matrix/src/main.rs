#![no_std]
#![no_main]

/* Just to link it in the executable (it provides the vector table) */
use defmt_rtt as _;
use panic_probe as _;
use stm32l4xx_hal::{pac, prelude::*};
use tp_led_matrix::{Color, Image};
use dwt_systick_monotonic::DwtSystick;
use stm32l4xx_hal::serial::{Config, Event, Rx, Serial};
use heapless::pool::{Box, Node, Pool};
use core::mem::MaybeUninit;



mod matrix;
use matrix::Matrix;


#[rtic::app(device = pac, dispatchers = [USART2, USART3])]
mod app {
    use super::*;
    use core::mem::swap;

    #[shared]
    struct Shared {
        next_image: Option<Box<Image>>,
        pool: Pool<Image>,
    }

    #[local]
    struct Local {
        matrix: Matrix,
        usart1_rx: Rx<pac::USART1>,
        current_image: Box<Image>,
        rx_image: Box<Image>,
    }

    #[monotonic(binds = SysTick, default = true)]
    type MyMonotonic = DwtSystick<80_000_000>;
    type Instant = <MyMonotonic as rtic::Monotonic>::Instant;


    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("defmt correctly initialized");

        let mut _cp = cx.core;
        let dp = cx.device;

        // Get high-level representations of hardware modules
        let mut rcc = dp.RCC.constrain();
        let mut flash = dp.FLASH.constrain();
        let mut pwr = dp.PWR.constrain(&mut rcc.apb1r1);

        let mut gpioa = dp.GPIOA.split(&mut rcc.ahb2);
        let mut gpiob = dp.GPIOB.split(&mut rcc.ahb2);
        let mut gpioc = dp.GPIOC.split(&mut rcc.ahb2);
        let mut mono = DwtSystick::new(&mut _cp.DCB, _cp.DWT, _cp.SYST, 80_000_000);

        // Setup the clocks at 80MHz using HSI (by default since HSE/MSI are not configured).
        // The flash wait states will be configured accordingly.
        let clocks = rcc.cfgr.sysclk(80.MHz()).freeze(&mut flash.acr, &mut pwr);

        let matrix = Matrix::new(
            gpioa.pa2,
            gpioa.pa3,
            gpioa.pa4,
            gpioa.pa5,
            gpioa.pa6,
            gpioa.pa7,
            gpioa.pa15,
            gpiob.pb0,
            gpiob.pb1,
            gpiob.pb2,
            gpioc.pc3,
            gpioc.pc4,
            gpioc.pc5,
            &mut gpioa.moder,
            &mut gpioa.otyper,
            &mut gpiob.moder,
            &mut gpiob.otyper,
            &mut gpioc.moder,
            &mut gpioc.otyper,
            clocks,
        );
        // Configure PB6 and PB7 into the right mode

        let serial_tx =
            gpiob
                .pb6
                .into_alternate::<7>(&mut gpiob.moder, &mut gpiob.otyper, &mut gpiob.afrl);
        let serial_rx =
            gpiob
                .pb7
                .into_alternate::<7>(&mut gpiob.moder, &mut gpiob.otyper, &mut gpiob.afrl);

        let config = Config::default().baudrate(38400.bps());

        let mut serial = Serial::usart1(
            dp.USART1,
            (serial_tx, serial_rx),
            config,
            clocks,
            &mut rcc.apb2,
        );
        serial.listen(Event::Rxne);
        let usart1_rx = serial.split().1;

        let pool: Pool<Image> = Pool::new();
        unsafe {
            static mut MEMORY: core::mem::MaybeUninit<[Node<Image> ;3]>  = MaybeUninit::uninit();
            pool.grow_exact(&mut MEMORY); // static mut access is unsafe
        }

        let current_image = pool.alloc().unwrap().init(Image::default());
        let rx_image = pool.alloc().unwrap().init(Image::default());
        let next_image = None;

        display::spawn(mono.now()).unwrap();

        // Return the resources and the monotonic timer
        (
            Shared { pool, next_image },
            Local { matrix, usart1_rx, current_image, rx_image},
            init::Monotonics(mono),
        )
    }

    #[idle(local = [counter: u32 = 0])]
        fn idle(cx: idle::Context) -> ! {
            loop {
                if *cx.local.counter == 10_000 {
                    defmt::info!("Idle task ran a lot !");
                    *cx.local.counter += 1;
                } else {
                    *cx.local.counter = 0;
                }
        }
    }

    #[task(local = [matrix, next_line: usize = 0, current_image], shared = [next_image, pool], priority = 2)]    fn display(cx: display::Context, at: Instant) {
        // Display line next_line (cx.local.next_line) of
        // the image (cx.local.image) on the matrix (cx.local.matrix).
        // All those are mutable references.

        cx.local.matrix.send_row(*cx.local.next_line, cx.local.current_image.row(*cx.local.next_line));

        // Increment next_line up to 7 and wraparound to 0

        if *cx.local.next_line == 7 {
            *cx.local.next_line = 0;
            /*  if next_image contains an image, take() it in a variable image and swap()
            it with current_image. Return the old image (which is now in image after the swap) to the pool. */
            (cx.shared.next_image, cx.shared.pool).lock(|next_image, pool| {

                if let Some(mut contain_image) = next_image.take() {
                    swap(&mut contain_image, cx.local.current_image);
                    pool.free(contain_image);
                }
            });
        }
        else {
            *cx.local.next_line = *cx.local.next_line + 1;
        }

        let period = (1.secs() / 8) / 60;
        let next_time: Instant = at + period;

        display::spawn_at(next_time, next_time).unwrap();
    } 

    #[task(binds = USART1,
		local = [usart1_rx, rx_image, next_pos: usize = 0, begin: bool = false],
		shared = [next_image, pool])]
        fn receive_byte(cx: receive_byte::Context) {
            let rx_image = cx.local.rx_image;
            let next_position: &mut usize = cx.local.next_pos;
            const SIZE: usize = 8 * 8 * 3;

            if let Ok(b) = cx.local.usart1_rx.read() {
                // Handle the incoming byte according to the SE203 protocol
                // and update next_image
                // Do not forget that next_image.as_mut() might be handy here!
                defmt::debug!("Received byte {:x}", b);

                if b == 0xff {
                    *next_position = 0;
                    *cx.local.begin = true;
                    defmt::debug!("Begin new image");
                } else if *cx.local.begin {
                    rx_image.as_mut()[*next_position] = b;
                }

                // If the received image is complete, make it available to
                // the display task.
                *next_position += 1;
                if *next_position == SIZE {
                    (cx.shared.next_image, cx.shared.pool).lock(|next_image, pool| {
                        // Replace the image content by the new one, for example
                        // by swapping them, and reset next_pos
                        if let Some(image) = next_image.take() {
                            pool.free(image);
                        }
                        let mut future_image = pool.alloc().unwrap().init(Image::default());
                        swap(&mut future_image, rx_image);
                        *next_image = Some(future_image);

                        *next_position = 0;
                        *cx.local.begin = false;
                    });
                }
            }
    }


    
    

}