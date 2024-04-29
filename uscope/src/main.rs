use petri::{Cell, Dish, CHUNK_SIZE};

fn main() {
	let mut dish = Dish::new();
	loop {
		for _ in 0..1000 {
			dish.fire_blindly();
		}
		print_dish(&dish);
		wait_for_input()
	}
}

fn print_dish(dish: &Dish) {
	for y in 0..(CHUNK_SIZE / 2) {
		for x in 0..CHUNK_SIZE {
			render_pixel_pair(dish, x, y);
		}
		println!();
	}
	println!();
}

fn render_pixel_pair(dish: &Dish, x: usize, y: usize) {
	let a = dish.get_cell(x, y * 2) != Some(Cell::EMPTY);
	let b = dish.get_cell(x, y * 2 + 1) != Some(Cell::EMPTY);
	let char = match (a, b) {
		(false, false) => " ",
		(false, true) => "▄",
		(true, false) => "▀",
		(true, true) => "█",
	};
	print!("{}", char);
}

pub fn wait_for_input() {
	std::io::stdin().read_line(&mut String::new()).unwrap();
}
