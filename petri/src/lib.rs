use std::ops::Not;

use rand::prelude::*;
use serde::{Deserialize, Serialize};

pub const CHUNK_SIZE: usize = 32;

#[derive(Default, Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct Cell(pub u16);

#[derive(Debug, Serialize, Deserialize)]
pub struct Dish {
	#[serde(skip)]
	world: World,
	pub rules: Vec<Rule>, // todo make read-only to ensure cache is updated
	pub types: Vec<CellData>,
	pub groups: Vec<CellGroup>, // todo make read-only to ensure cache is updated
	#[serde(skip)]
	cache: Vec<RuleCache>,
	#[serde(skip)]
	match_cache: Vec<usize>,
}

#[derive(Debug)]
struct RuleCache {
	rule: usize,
	variant: usize,
	matches: Vec<(isize, isize)>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CellGroup {
	pub name: String,
	pub void: bool,
	pub cells: Vec<Cell>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CellData {
	pub name: String,
	pub color: [u8; 3],
}

#[derive(Debug, Default)]
struct Chunk {
	pub contents: Box<[[Cell; CHUNK_SIZE]; CHUNK_SIZE]>,
}

#[derive(Debug, Default)]
struct World {
	chunk: Chunk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
	#[serde(default)]
	pub name: String,
	base: SubRule,
	#[serde(skip)]
	variants: Vec<SubRule>,
	pub enabled: bool,
	pub flip_x: bool,
	pub flip_y: bool,
	pub rotate: bool,
	#[serde(default)]
	pub failrate: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SubRule {
	width: usize,
	height: usize,
	contents: Vec<(RuleCellFrom, RuleCellTo)>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleCellFrom {
	/// matches anything
	#[default]
	Any,
	/// matches one cell type
	One(Cell),
	/// matches anything defined in the group referenced by this index
	Group(usize),
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleCellTo {
	/// don't modify this cell
	#[default]
	None,
	/// set to this cell
	One(Cell),
	/// randomly choose from the group
	GroupRandom(usize),
	/// copy the cell from the corresponding input position
	Copy(usize, usize),
}

impl SubRule {
	fn new() -> Self {
		Self {
			width: 1,
			height: 1,
			contents: vec![Default::default()],
		}
	}

	fn get(&self, x: usize, y: usize) -> (RuleCellFrom, RuleCellTo) {
		if x >= self.width || y >= self.height {
			Default::default()
		} else {
			self.contents[x + self.width * y].clone()
		}
	}

	fn get_mut(&mut self, x: usize, y: usize) -> &mut (RuleCellFrom, RuleCellTo) {
		assert!(x < self.width || y < self.height);
		&mut self.contents[x + self.width * y]
	}

	fn set_both(&mut self, x: usize, y: usize, cells: (RuleCellFrom, RuleCellTo)) {
		if x < self.width && y < self.height {
			self.contents[x + self.width * y] = cells;
		}
	}

	fn set_from(&mut self, x: usize, y: usize, cell: RuleCellFrom) {
		if x < self.width && y < self.height {
			self.contents[x + self.width * y].0 = cell;
		}
	}

	fn set_to(&mut self, x: usize, y: usize, cell: RuleCellTo) {
		if x < self.width && y < self.height {
			self.contents[x + self.width * y].1 = cell;
		}
	}
}

type ResizeParam = (isize, isize, isize, isize);
impl Rule {
	pub const EXTEND_LEFT: ResizeParam = (1, 0, -1, 0);
	pub const EXTEND_RIGHT: ResizeParam = (1, 0, 0, 0);
	pub const EXTEND_UP: ResizeParam = (0, 1, 0, -1);
	pub const EXTEND_DOWN: ResizeParam = (0, 1, 0, 0);
	pub const SHRINK_LEFT: ResizeParam = (-1, 0, 1, 0);
	pub const SHRINK_RIGHT: ResizeParam = (-1, 0, 0, 0);
	pub const SHRINK_UP: ResizeParam = (0, -1, 0, 1);
	pub const SHRINK_DOWN: ResizeParam = (0, -1, 0, 0);

	pub fn new() -> Self {
		Self {
			name: "new rule".into(),
			enabled: false,
			base: SubRule::new(),
			variants: vec![SubRule::new()],
			flip_x: false,
			flip_y: false,
			rotate: false,
			failrate: 0,
		}
	}

	pub fn get(&self, x: usize, y: usize) -> (RuleCellFrom, RuleCellTo) {
		self.base.get(x, y)
	}

	pub fn get_mut(&mut self, x: usize, y: usize) -> &mut (RuleCellFrom, RuleCellTo) {
		self.base.get_mut(x, y)
	}

	pub fn set_from(&mut self, x: usize, y: usize, cell: RuleCellFrom) {
		self.base.set_from(x, y, cell);
		self.generate_variants();
	}

	pub fn set_to(&mut self, x: usize, y: usize, cell: RuleCellTo) {
		self.base.set_to(x, y, cell);
		self.generate_variants();
	}

	pub fn height(&self) -> usize {
		self.base.height
	}

	pub fn width(&self) -> usize {
		self.base.width
	}

	pub fn resize(&mut self, params: ResizeParam) {
		let (dw, dh, dx, dy) = params;

		let new_width = self.base.width.saturating_add_signed(dw);
		let new_height = self.base.height.saturating_add_signed(dh);
		if new_width < 1 || new_height < 1 {
			return;
		}
		let mut new_contents = vec![Default::default(); new_width * new_height];

		for nx in 0..new_width {
			let oldx = nx.wrapping_add_signed(dx);
			for ny in 0..new_height {
				let oldy = ny.wrapping_add_signed(dy);
				new_contents[nx + new_width * ny] = self.get(oldx, oldy);
			}
		}

		self.base.contents = new_contents;
		self.base.height = new_height;
		self.base.width = new_width;
		self.generate_variants();
	}

	pub fn variant_count(&self) -> usize {
		self.variants.len()
	}

	pub fn generate_variants(&mut self) {
		self.variants.clear();
		self.variants.push(self.base.clone());

		fn transform_variants(variants: &mut Vec<SubRule>, f: fn(&SubRule) -> SubRule) {
			let mut new = Vec::new();
			for v in variants.iter() {
				let new_variant = f(v);
				if !variants.contains(&new_variant) {
					new.push(new_variant);
				}
			}
			variants.extend_from_slice(&new);
		}

		if self.flip_x {
			transform_variants(&mut self.variants, |b| {
				let mut new = b.clone();
				for y in 0..new.height {
					for x in 0..new.width {
						let mut old = b.get(new.width - x - 1, y);
						if let (_, RuleCellTo::Copy(cx, _cy)) = &mut old {
							*cx = new.width - *cx - 1;
						}
						new.set_both(x, y, old);
					}
				}
				new
			});
		}
		if self.flip_y {
			transform_variants(&mut self.variants, |b| {
				let mut new = b.clone();
				for y in 0..new.height {
					for x in 0..new.width {
						let mut old = b.get(x, new.height - y - 1);
						if let (_, RuleCellTo::Copy(_cx, cy)) = &mut old {
							*cy = new.height - *cy - 1;
						}
						new.set_both(x, y, old);
					}
				}
				new
			});
		}
		if self.rotate {
			// 180° rotations (same as flipping x and y)
			transform_variants(&mut self.variants, |b| {
				let mut new = b.clone();
				for y in 0..new.height {
					for x in 0..new.width {
						let mut old = b.get(new.width - x - 1, new.height - y - 1);
						if let (_, RuleCellTo::Copy(cx, cy)) = &mut old {
							*cx = new.width - *cx - 1;
							*cy = new.height - *cy - 1;
						}
						new.set_both(x, y, old);
					}
				}
				new
			});
			// 90° rotations
			transform_variants(&mut self.variants, |b| {
				let mut new = b.clone();
				new.height = b.width;
				new.width = b.height;
				for y in 0..new.height {
					for x in 0..new.width {
						let mut old = b.get(y, x);
						if let (_, RuleCellTo::Copy(cx, cy)) = &mut old {
							(*cx, *cy) = (*cy, *cx);
						}
						new.set_both(x, y, old);
					}
				}
				new
			})
		}
	}
}

impl Chunk {
	fn new() -> Self {
		Self {
			contents: vec![[Cell(0); CHUNK_SIZE]; CHUNK_SIZE]
				.into_boxed_slice()
				.try_into()
				.unwrap(),
		}
	}

	fn fill(&mut self, cell: Cell) {
		self.contents.fill([cell; CHUNK_SIZE]);
	}

	fn with_random_ones(mut self) -> Self {
		for col in self.contents.iter_mut() {
			for cell in col.iter_mut() {
				if random::<u8>() % 4 == 0 {
					*cell = Cell(1);
				}
			}
		}
		self
	}

	fn get_cell(&self, x: usize, y: usize) -> Cell {
		self.contents[x][y]
	}

	fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
		self.contents[x][y] = cell
	}
}

impl Dish {
	pub fn new() -> Self {
		let mut default_rules = vec![
			Rule {
				enabled: true,
				name: "fall".into(),
				base: SubRule {
					width: 1,
					height: 2,
					contents: vec![
						(RuleCellFrom::One(Cell(1)), RuleCellTo::One(Cell(0))),
						(RuleCellFrom::One(Cell(0)), RuleCellTo::One(Cell(1))),
					],
				},
				..Rule::new()
			},
			Rule {
				enabled: true,
				name: "slide".into(),
				base: SubRule {
					width: 2,
					height: 2,
					contents: vec![
						(RuleCellFrom::One(Cell(1)), RuleCellTo::One(Cell(0))),
						(RuleCellFrom::Any, RuleCellTo::None),
						(RuleCellFrom::One(Cell(1)), RuleCellTo::None),
						(RuleCellFrom::One(Cell(0)), RuleCellTo::One(Cell(1))),
					],
				},
				flip_x: true,
				..Rule::new()
			},
		];

		for rule in &mut default_rules {
			rule.generate_variants()
		}

		Self {
			world: World {
				chunk: Chunk::new().with_random_ones(),
			},
			rules: default_rules,
			types: vec![
				CellData::new("air", 0, 0, 0),
				CellData::new("pink_sand", 255, 147, 219),
			],
			groups: vec![CellGroup {
				name: "empty".into(),
				void: true,
				cells: vec![Cell(0)],
			}],
			cache: Vec::new(),
			match_cache: Vec::new(),
		}
	}

	pub fn fill(&mut self, cell: Cell) {
		self.world.fill(cell);
	}

	pub fn update_all_rules(&mut self) {
		for rule in &mut self.rules {
			rule.generate_variants();
		}
		self.rebuild_cache();
	}

	/// run after any rule modification
	pub fn update_cache_single_rule(&mut self, rule_index: usize) {
		// remove old cache for this rule, since the variants may have changed
		self.cache.retain(|c| c.rule != rule_index);
		self.add_cache_single_rule(rule_index);
		self.update_match_cache();
	}

	/// run after adding a rule
	pub fn cache_last_added_rule(&mut self) {
		if self.rules.is_empty() {
			return;
		}
		let index = self.rules.len() - 1;
		self.update_cache_single_rule(index);
	}

	fn add_cache_single_rule(&mut self, rule_index: usize) {
		let rule = &self.rules[rule_index];
		if !rule.enabled {
			return;
		}
		for variant_index in 0..rule.variants.len() {
			let mut matches = Vec::new();

			let rule = &rule.variants[variant_index];
			let border_x = rule.width as isize - 1;
			let border_y = rule.height as isize - 1;

			for px in -border_x..(CHUNK_SIZE as isize) {
				for py in -border_y..(CHUNK_SIZE as isize) {
					if self.world.subrule_matches(px, py, &rule, &self.groups) {
						matches.push((px, py));
					}
				}
			}
			self.cache.push(RuleCache {
				rule: rule_index,
				variant: variant_index,
				matches,
			});
		}
	}

	pub fn rebuild_cache(&mut self) {
		self.cache.clear();
		for rule_index in 0..self.rules.len() {
			self.add_cache_single_rule(rule_index);
		}
		self.update_match_cache();
	}

	pub fn update_cache(&mut self, x: isize, y: isize, width: usize, height: usize) {
		fn overlap(
			(x1, y1, w1, h1): (isize, isize, usize, usize),
			(x2, y2, w2, h2): (isize, isize, usize, usize),
		) -> bool {
			x2 < x1.saturating_add_unsigned(w1)
				&& x1 < x2.saturating_add_unsigned(w2)
				&& y2 < y1.saturating_add_unsigned(h1)
				&& y1 < y2.saturating_add_unsigned(h2)
		}
		let edited_rect = (x, y, width, height);

		for cache in &mut self.cache {
			let rule = &self.rules[cache.rule].variants[cache.variant];
			let rule_width = rule.width;
			let rule_height = rule.height;

			// discard all overlapping matches
			let mut i = 0;
			while i < cache.matches.len() {
				let match_pos = cache.matches[i];
				let match_rect = (match_pos.0, match_pos.1, rule_width, rule_height);
				if overlap(edited_rect, match_rect) {
					cache.matches.swap_remove(i);
				} else {
					i += 1;
				}
			}
			// check entire changed area and add matches
			let border_x = rule_width - 1;
			let border_y = rule_height - 1;

			for px in (x.wrapping_sub_unsigned(border_x))..(x.wrapping_add_unsigned(width)) {
				for py in (y.wrapping_sub_unsigned(border_y))..(y.wrapping_add_unsigned(height)) {
					if self.world.subrule_matches(px, py, &rule, &self.groups) {
						cache.matches.push((px, py));
					}
				}
			}
		}
		self.update_match_cache();
	}

	fn update_match_cache(&mut self) {
		self.match_cache = self
			.cache
			.iter()
			.enumerate()
			.filter_map(|(i, c)| c.matches.is_empty().not().then_some(i))
			.collect();
	}

	pub fn fire_once(&mut self) {
		if self.match_cache.is_empty() {
			return;
		}
		let i = random::<usize>() % self.match_cache.len();
		let i = self.match_cache[i];
		let rule_cache = &self.cache[i];
		let match_pos_index = random::<usize>() % rule_cache.matches.len();
		let (x, y) = rule_cache.matches[match_pos_index];

		let rule = &self.rules[rule_cache.rule].variants[rule_cache.variant];
		let width = rule.width;
		let height = rule.height;

		self.apply_rule(x, y, rule_cache.rule, rule_cache.variant);
		self.update_cache(x, y, width, height);
	}

	pub fn fire_blindly(&mut self) {
		if self.rules.is_empty() {
			return;
		}
		let enabled_rules = self
			.rules
			.iter()
			.enumerate()
			.filter_map(|(i, r)| r.enabled.then_some(i))
			.collect::<Vec<_>>();
		if enabled_rules.is_empty() {
			return;
		}
		let enabled_rule_index = random::<usize>() % enabled_rules.len();
		let rule_index = enabled_rules[enabled_rule_index];
		let rule = &self.rules[rule_index];
		let variant_index = random::<usize>() % rule.variants.len();
		let variant = &rule.variants[variant_index].clone();
		let border_x = variant.width - 1;
		let border_y = variant.height - 1;
		let x = ((random::<usize>() % (CHUNK_SIZE + border_x)) as isize)
			.wrapping_sub_unsigned(border_x);
		let y = ((random::<usize>() % (CHUNK_SIZE + border_y)) as isize)
			.wrapping_sub_unsigned(border_y);

		if self.world.subrule_matches(x, y, variant, &self.groups) {
			self.apply_rule(x, y, rule_index, variant_index);
		}
	}

	fn apply_rule(&mut self, x: isize, y: isize, rule_index: usize, variant_index: usize) {
		let rule = &self.rules[rule_index];
		let variant = &rule.variants[variant_index].clone();

		if rule.failrate > random() {
			return;
		}

		let width = variant.width;
		let height = variant.height;

		let mut old_state = Vec::new();
		for dy in 0..height {
			for dx in 0..width {
				old_state.push(
					self.get_cell((x as usize).wrapping_add(dx), (y as usize).wrapping_add(dy)),
				);
			}
		}

		for dx in 0..width {
			for dy in 0..height {
				let px = x.wrapping_add_unsigned(dx) as usize;
				let py = y.wrapping_add_unsigned(dy) as usize;
				match variant.get(dx, dy).1 {
					RuleCellTo::One(rule_cell) => {
						self.set_cell(px, py, rule_cell);
					}
					RuleCellTo::GroupRandom(group_id) => {
						let group = &self.groups[group_id];
						if !group.cells.is_empty() {
							let i = random::<usize>() % group.cells.len();
							let cell = group.cells[i];
							self.set_cell(px, py, cell);
						}
					}
					RuleCellTo::Copy(x, y) => {
						let index = x + y * variant.width;
						if index >= old_state.len() {
							// TODO sanitize the rules somewhere else and remove this bounds check
							// the copy source is outside the rule bounds
							continue;
						}
						let cell = old_state[index];
						if let Some(cell) = cell {
							// if the copy source is outside the world, do nothing
							self.set_cell(px, py, cell);
						}
					}
					RuleCellTo::None => (),
				}
			}
		}
	}

	//todo isize
	pub fn get_cell(&self, x: usize, y: usize) -> Option<Cell> {
		self.world.get_cell(x, y)
	}

	//todo isize
	pub fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
		if x >= CHUNK_SIZE || y >= CHUNK_SIZE {
			return;
		}
		self.world.chunk.set_cell(x, y, cell);
	}
}

impl World {
	fn fill(&mut self, cell: Cell) {
		self.chunk.fill(cell);
	}

	//todo isize
	fn get_cell(&self, x: usize, y: usize) -> Option<Cell> {
		if x >= CHUNK_SIZE || y >= CHUNK_SIZE {
			None
		} else {
			Some(self.chunk.get_cell(x, y))
		}
	}

	fn subrule_matches(&self, x: isize, y: isize, subrule: &SubRule, groups: &[CellGroup]) -> bool {
		for dx in 0..subrule.width {
			for dy in 0..subrule.height {
				let x = x.wrapping_add_unsigned(dx) as usize;
				let y = y.wrapping_add_unsigned(dy) as usize;
				let cell = self.get_cell(x, y);
				match subrule.get(dx, dy).0 {
					RuleCellFrom::One(rule_cell) => {
						if cell != Some(rule_cell) {
							return false;
						}
					}
					RuleCellFrom::Group(group_id) => {
						let group = &groups[group_id];
						if let Some(cell) = cell {
							if !group.cells.contains(&cell) {
								return false;
							}
						} else if !group.void {
							return false;
						}
					}
					RuleCellFrom::Any => (),
				}
			}
		}
		true
	}
}

impl Cell {
	pub fn id(&self) -> usize {
		self.0 as usize
	}
}

impl CellData {
	pub fn new(name: &str, r: u8, g: u8, b: u8) -> Self {
		Self {
			name: name.to_owned(),
			color: [r, g, b],
		}
	}
}
