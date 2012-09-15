use io::WriterUtil;
use std::map::*;
use core::ops::*;

export Unit, Meter, Feet, Second, Minute, Micro, Milli, Kilo, Compound,
	Value;

// ---- Unit ----------------------------------------------------------------------------
/// Simple units are specified with one of the constructors (e.g. Meter).
/// Compound units are constructed using multiplication and division
/// (e.g. Meter/(Second*Second)). Dimensionless units are empty Compound
/// units.
enum Unit
{
	// length
	Meter,
	Feet,
	
	// time
	Second,
	Minute,
	
	// modifiers
	Micro,
	Milli,
	Kilo,
	
	// compound
	Compound(@[Unit], @[Unit]),	// numerator, denominator (which must be simple units)
}

impl Unit : ops::Mul<Unit, Unit>
{
	pure fn mul(rhs: Unit) -> Unit
	{
		let (numer1, denom1) = to_compound(self);
		let (numer2, denom2) = to_compound(rhs);
		cancel_units(numer1 + numer2, denom1 + denom2)
	}
}

impl Unit : ops::Div<Unit, Unit>
{
	pure fn div(rhs: Unit) -> Unit
	{
		let (numer1, denom1) = to_compound(self);
		let (numer2, denom2) = to_compound(rhs);
		cancel_units(numer1 + denom2, denom1 + numer2)	// division is multiply by reciprocal
	}
}

impl  Unit : ToStr 
{
	fn to_str() -> ~str
	{
		match self
		{
			// length
			Meter		=> ~"m",
			Feet		=> ~"ft",
			
			// time
			Second		=> ~"s",
			Minute		=> ~"min",
			
			// modifiers
			Micro		=> ~"u",
			Milli		=> ~"m",
			Kilo		=> ~"k",
			
			// compound
			Compound(n, d) if  n.is_empty() && d.is_empty() =>
			{
				~""
			}
			Compound(n, d) =>
			{
				// TODO: need to report x*x as x^2
				// TODO: need to special case empty arrays
				let numer = str::connect(do n.map |u| {u.to_str()}, ~"*");
				let denom = str::connect(do d.map |u| {u.to_str()}, ~"*");
				fmt!("%s/%s", numer, denom)
			}
		}
	}
}

// TODO: This is hopefully temporary: at some point rust should again be able to compare enums without assistence.
impl Unit : cmp::Eq
{
	pure fn eq(&&rhs: Unit) -> bool
	{
		fmt!("%?", self) == fmt!("%?", rhs)
	}
	
	pure fn ne(&&rhs: Unit) -> bool
	{
		fmt!("%?", self) != fmt!("%?", rhs)
	}
}

// ---- Value ---------------------------------------------------------------------------
/// Values are numbers represented in an arbitrary unit. They support
/// the standard arithmetic operations and fail is called if the units are
/// incommensurable (e.g. if meters are added to seconds).
///
/// Note that units are converted to different units only when explicitly
/// directed to do so (e.g. via convert_to). 
struct Value
{
	pub value: float,
	priv units: Unit,		// private so that we can enforce the invariant that Compound units only contain simple units
}

/// Creates a dimensionless value.
pure fn from_number(value: float) -> Value
{
	Value {value: value, units: Compound(@[], @[])}
}

pure fn from_units(value: float, units: Unit) -> Value
{
	match units
	{
		Compound(u, v) if u.len() == 1 && v.is_empty() =>
		{
			Value {value: value, units: u[0]}
		}
		u =>
		{
			Value {value: value, units: u}
		}
	}
}

impl Value
{
	fn convert_to(to: Unit) -> Value
	{
		check_commensurable(self.units, to, ~"convert_to");
		let c = to_canonical(self);
		from_canonical(c.value, to)
	}
}

impl Value : ops::Mul<Value, Value>
{
	pure fn mul(rhs: Value) -> Value
	{
		Value {value: self.value*rhs.value, units: self.units*rhs.units}
	}
}

impl Value : ops::Div<Value, Value>
{
	pure fn div(rhs: Value) -> Value
	{
		Value {value: self.value/rhs.value, units: self.units/rhs.units}
	}
}

// ---- Internal Items ------------------------------------------------------------------
pure fn to_compound(unit: Unit) -> (@[Unit], @[Unit])
{
	match unit
	{
		Compound(n, d)	=> (n, d),
		u					=> (@[u], @[]),
	}
}

pure fn cancel_units(numer: @[Unit], denom: @[Unit]) -> Unit
{
	pure fn box_remove_at<T: Copy>(v: @[T], index: uint) -> @[T]
	{
		do at_vec::build_sized(v.len() - 1)
		|push|
		{
			for v.eachi
			|i, e|
			{
				if i != index
				{
					push(e);
				}
			}
		}
	}
	
	let mut rnumer = @[];
	let mut rdenom = copy denom;
	
	for numer.each
	|u|
	{
		match denom.position_elem(u)
		{
			option::Some(i)	=> rdenom = box_remove_at(rdenom, i),
			option::None		=> rnumer += @[u],
		}
	}
	
	if rnumer.len() == 1 && rdenom.is_empty()
	{
		rnumer[0]
	}
	else
	{
		Compound(rnumer, rdenom)
	}
}

pure fn to_canonical(x: Value) -> Value
{
	let mut rvalue = x.value;
	let mut rnumer = @[];
	let mut rdenom = @[];
	
	let (numer, denom) =
		match x.units
		{
			Compound(n, d)	=> (n, d),
			u					=> (@[u], @[]),
		};
	
	for numer.each
	|u|
	{
		let (scaling, v) = canonical_unit(u);
		rvalue = rvalue*scaling;
		rnumer += v;
	}
	
	for denom.each
	|u|
	{
		let (scaling, v) = canonical_unit(u);
		rvalue = rvalue*(1.0/scaling);
		rdenom += v;
	}
	
	from_units(rvalue, Compound(rnumer, rdenom))
}

pure fn from_canonical(x: float, u: Unit) -> Value
{
	let mut rvalue = x;
	let mut rnumer = @[];
	let mut rdenom = @[];
	
	let (numer, denom) =
		match u
		{
			Compound(n, d)	=> (n, d),
			u					=> (@[u], @[]),
		};
	
	for numer.each
	|u|
	{
		let (scaling, _v) = canonical_unit(u);
		rvalue = rvalue*(1.0/scaling);
		rnumer += @[u];
	}
	
	for denom.each
	|u|
	{
		let (scaling, _v) = canonical_unit(u);
		rvalue = rvalue*scaling;
		rdenom += @[u];
	}
	
	from_units(rvalue, Compound(rnumer, rdenom))
}

fn check_commensurable(lhs: Unit, rhs: Unit, fname: ~str)
{
	let numer1 = box_str_hash();
	let denom1 = box_str_hash();
	increment_type(numer1, denom1, lhs);
	
	let numer2 = box_str_hash();
	let denom2 = box_str_hash();
	increment_type(numer2, denom2, rhs);
	
	if numer1 != numer2 || denom1 != denom2
	{
		fail fmt!("incommensurable units for `%s`.%s(`%s`)", lhs.to_str(), fname, rhs.to_str());
	}
}

fn increment_type(numer: hashmap<@~str, uint>, denom: hashmap<@~str, uint>, u: Unit)
{
	fn increment(table: hashmap<@~str, uint>, u: Unit)
	{
		let key =
			match u
			{
				Meter | Feet			=> @~"length",
				Second | Minute		=> @~"time",
				Micro | Milli | Kilo		=> return,
				Compound(*)			=> fail fmt!("%? should not be nested", u),
			};
		match table.find(key)
		{
			option::Some(count)	=> table.insert(key, count + 1),
			option::None			=> table.insert(key, 1),
		};
	}
	
	match u
	{
		Compound(n, d)	=>
		{
			for n.each |v| {increment(numer, v)}
			for d.each |v| {increment(denom, v)}
		}
		_ => {increment(numer, u)}
	}
}

pure fn canonical_unit(u: Unit) -> (float, @[Unit])
{
	match u
	{
		// length
		Meter			=> (1.0, @[Meter]),
		Feet			=> (0.3048, @[Meter]),
		
		// time
		Second			=> (1.0, @[Second]),
		Minute			=> (60.0, @[Second]),
		
		// modifiers
		Micro			=> (1.0e-6, @[]),
		Milli			=> (1.0e-3, @[]),
		Kilo			=> (1.0e3, @[]),
		
		// compound
		Compound(*)	=> fail fmt!("Expected a simple unit but found %?", u),
	}
}

// ---- Tests ---------------------------------------------------------------------------
#[cfg(test)]
fn check_floats(actual: float, expected: float) -> bool
{
	if float::abs(actual - expected) > 0.001
	{
		io::stderr().write_line(fmt!("Found %f but expected %f", actual, expected));
		return false;
	}
	return true;
}

#[cfg(test)]
fn check_units(actual: Unit, expected: Unit) -> bool
{
	if fmt!("%?", actual) != fmt!("%?", expected)	// TODO: use != when enums again support equality
	{
		io::stderr().write_line(fmt!("Found %? but expected %?", actual, expected));
		return false;
	}
	return true;
}

#[test]
fn test_mul_unit()
{
	assert check_units(Meter*Meter, Compound(@[Meter, Meter], @[]));
	assert check_units(Kilo*Second, Compound(@[Kilo, Second], @[]));
	assert check_units((Meter*Meter)*(Second*Second), Compound(@[Meter, Meter, Second, Second], @[]));
}

#[test]
fn test_div_unit()
{
	assert check_units(Meter/Second, Compound(@[Meter], @[Second]));
	assert check_units(Second*Meter/Second, Meter);
	assert check_units(Second*(Meter/Second), Meter);
	assert check_units(Second*Meter/(Meter*Second*Second), Compound(@[], @[Second]));
}

#[test]
fn test_mul_value()
{
	let x = from_number(5.0)*from_units(3.0, Meter);
	assert check_floats(x.value, 15.0);
	assert check_units(x.units, Meter);
	
	let x = from_units(5.0, Meter)*from_units(3.0, Meter);
	assert check_floats(x.value, 15.0);
	assert check_units(x.units, Compound(@[Meter, Meter], @[]));
}

#[test]
fn test_div_value()
{
	let x = from_number(5.0)/from_units(2.0, Meter);
	assert check_floats(x.value, 2.5);
	assert check_units(x.units, Compound(@[], @[Meter]));
	
	let x = from_units(5.0, Meter)/from_units(2.0, Meter);
	assert check_floats(x.value, 2.5);
	assert check_units(x.units, Compound(@[], @[]));
	
	let x = from_units(5.0, Meter*Second)/from_units(2.0, Second);
	assert check_floats(x.value, 2.5);
	assert check_units(x.units, Meter);
	
	let x = from_units(5.0, Meter)/from_units(2.0, Second);
	assert check_floats(x.value, 2.5);
	assert check_units(x.units, Compound(@[Meter], @[Second]));
	
	let x = from_units(5.0, Meter)/from_units(2.0, Meter*Second);
	assert check_floats(x.value, 2.5);
	assert check_units(x.units, Compound(@[], @[Second]));
}

#[test]
fn test_to_canonical()
{
	let x = to_canonical(from_units(3.0, Meter));
	assert check_floats(x.value, 3.0);
	assert check_units(x.units, Meter);
	
	let x = to_canonical(from_units(3.0, Feet));
	assert check_floats(x.value, 0.9144);
	assert check_units(x.units, Meter);
	
	let x = to_canonical(from_units(3.0, Kilo*Meter));
	assert check_floats(x.value, 3000.0);
	assert check_units(x.units, Meter);
	
	let x = to_canonical(from_units(3.0, Kilo*Feet));
	assert check_floats(x.value, 914.4);
	assert check_units(x.units, Meter);
	
	let x = to_canonical(from_units(3.0, Feet)/from_units(2.0, Minute));
	assert check_floats(x.value, 0.00762);
	assert check_units(x.units, Compound(@[Meter], @[Second]));
}

#[test]
fn test_from_canonical()
{
	let x = from_canonical(3.0, Meter);
	assert check_floats(x.value, 3.0);
	assert check_units(x.units, Meter);
	
	let x = from_canonical(3.0, Feet);
	assert check_floats(x.value, 9.8425197);
	assert check_units(x.units, Feet);
	
	let x = from_canonical(3000.0, Kilo*Meter);
	assert check_floats(x.value, 3.0);
	assert check_units(x.units, Compound(@[Kilo, Meter], @[]));
	
	let x = from_canonical(914.4, Kilo*Feet);
	assert check_floats(x.value, 3.0);
	assert check_units(x.units, Compound(@[Kilo, Feet], @[]));
}

#[test]
fn test_convert_to()
{
	let x = from_units(5.0, Feet).convert_to(Meter);
	assert check_floats(x.value, 1.524);
	assert check_units(x.units, Meter);
	
	let x = from_units(5.0, Kilo*Meter).convert_to(Feet);
	assert check_floats(x.value, 16_404.199);
	assert check_units(x.units, Feet);
	
	let x = from_units(5.0, Kilo*Meter).convert_to(Milli*Feet);
	assert check_floats(x.value, 16_404_199.475);
	assert check_units(x.units, Milli*Feet);
	
	let x = from_units(5.0, Meter/Second).convert_to(Feet/Minute);
	assert check_floats(x.value, 984.25197);
	assert check_units(x.units, Feet/Minute);
	
	let x = from_units(5.0, Kilo*Meter*Second).convert_to(Second*Milli*Feet);
	assert check_floats(x.value, 16_404_199.475);
	assert check_units(x.units, Second*Milli*Feet);
}

#[test]
#[should_fail]
fn test_incommensurable_convert()
{
	let x = from_units(5.0, Feet).convert_to(Second);
	assert x.value > 0.0;
}

// TODO:
// better unit to_str
//    use exponentiation for repeated units
//	test 1/5m (prints 0.2m^-1)
//	test 1/5m*m (prints 0.2m^-2)
// value to_str
//	test 1/5m (prints 0.2m^-1)
//	test 1/5m*m (prints 0.2m^-2)
// add a script to generate tables (or a macro)
//    use SI units
//    use IEC binary prefixes
//    use imperial units
// support comparisons (need commensurate check)
// support plus/minus (need commensurate check)
// support modulo (need commensurate check)
// support neg
// maybe turn this into a project
