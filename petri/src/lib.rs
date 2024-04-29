use rand::prelude::*;

pub const CHUNK_SIZE: usize = 32;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Cell(u8, u8, u8);

#[derive(Debug)]
pub struct Pos2(i8, i8);

#[derive(Debug)]
pub struct Dish {
	pub chunk: Chunk,
	pub rules: Vec<Rule>,
}

#[derive(Debug)]
pub struct Chunk {
	pub contents: Box<[[Cell; CHUNK_SIZE]; CHUNK_SIZE]>,
}

#[derive(Debug)]
pub struct Rule {
	pub from: RulePattern,
	pub to: RulePattern,
	// probability
	// flip
	// rotate
}

impl Chunk {
	fn new() -> Self {
		Self {
			contents: vec![[Cell::EMPTY; CHUNK_SIZE]; CHUNK_SIZE]
				.into_boxed_slice()
				.try_into()
				.unwrap(),
		}
	}

	fn fill_random(mut self) -> Self {
		for col in self.contents.iter_mut() {
			for cell in col.iter_mut() {
				if random::<u8>() % 4 == 0 {
					*cell = Cell::WHITE;
				}
			}
		}
		self
	}

	fn get_cell(&self, x: usize, y: usize) -> Cell {
		self.contents[x][y].clone()
	}

	fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
		self.contents[x][y] = cell
	}
}

impl Dish {
	pub fn new() -> Self {
		Self {
			chunk: Chunk::new().fill_random(),
			rules: vec![
				Rule {
					from: RulePattern {
						width: 1,
						height: 2,
						contents: vec![Some(Cell::WHITE), Some(Cell::EMPTY)],
					},
					to: RulePattern {
						width: 1,
						height: 2,
						contents: vec![Some(Cell::EMPTY), Some(Cell::WHITE)],
					},
				},
				Rule {
					from: RulePattern {
						width: 2,
						height: 2,
						contents: vec![
							Some(Cell::WHITE),
							Some(Cell::EMPTY),
							Some(Cell::WHITE),
							Some(Cell::EMPTY),
						],
					},
					to: RulePattern {
						width: 2,
						height: 2,
						contents: vec![
							Some(Cell::EMPTY),
							Some(Cell::EMPTY),
							Some(Cell::WHITE),
							Some(Cell::WHITE),
						],
					},
				},
				Rule {
					from: RulePattern {
						width: 2,
						height: 2,
						contents: vec![
							Some(Cell::EMPTY),
							Some(Cell::WHITE),
							Some(Cell::EMPTY),
							Some(Cell::WHITE),
						],
					},
					to: RulePattern {
						width: 2,
						height: 2,
						contents: vec![
							Some(Cell::EMPTY),
							Some(Cell::EMPTY),
							Some(Cell::WHITE),
							Some(Cell::WHITE),
						],
					},
				},
			],
		}
	}

	pub fn fire_blindly(&mut self) {
		assert!(!self.rules.is_empty());
		let x = random::<usize>() % CHUNK_SIZE;
		let y = random::<usize>() % CHUNK_SIZE;
		let rule = random::<usize>() % self.rules.len();
		// let rule = &self.rules[rule];

		self.fire_rule(rule, x, y);
	}

	fn fire_rule(&mut self, rule_index: usize, x: usize, y: usize) {
		let rule = &self.rules[rule_index];
		let width = rule.to.width;
		let height = rule.to.height;
		// check is match
		for dx in 0..width {
			for dy in 0..height {
				let x = x + dx;
				let y = y + dy;
				if let Some(rule_cell) = rule.from.get(dx, dy) {
					if self.get_cell(x, y) != Some(rule_cell) {
						return;
					}
				}
			}
		}
		for dx in 0..width {
			for dy in 0..height {
				let x = x + dx;
				let y = y + dy;
				if let Some(rule_cell) = &self.rules[rule_index].to.get(dx, dy) {
					self.set_cell(x, y, rule_cell.clone());
				}
			}
		}
	}

	//todo isize
	pub fn get_cell(&self, x: usize, y: usize) -> Option<Cell> {
		if x >= CHUNK_SIZE || y >= CHUNK_SIZE {
			None
		} else {
			Some(self.chunk.get_cell(x, y))
		}
	}

	//todo isize
	pub fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
		if x >= CHUNK_SIZE || y >= CHUNK_SIZE {
			return;
		}
		self.chunk.set_cell(x, y, cell)
	}
}

impl Cell {
	pub const EMPTY: Self = Self(0, 0, 0);
	pub const WHITE: Self = Self(255, 255, 255);
}

#[derive(Debug)]
pub struct RulePattern {
	width: usize,
	height: usize,
	contents: Vec<Option<Cell>>,
}

impl RulePattern {
	fn get(&self, x: usize, y: usize) -> Option<Cell> {
		if x >= self.width || y >= self.height {
			None
		} else {
			// dbg!(x, y);
			self.contents[x + self.width * y].clone()
		}
	}
}
