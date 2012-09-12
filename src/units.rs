use io::WriterUtil;
use core::ops::*;

export Unit, Meter, Feet, Second, Minute, Micro, Milli, Kilo,
	Value;

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
}

struct Value
{
	priv value: float,
	priv numer: @[Unit],		// these will only contain canonical units
	priv denom: @[Unit],
}

fn Value(value: float) -> Value
{
	Value {value: value, numer: @[], denom: @[]}
}

impl Value
{
	pure fn get() -> float
	{
		self.value
	}
}

impl Value : ops::Mul<Unit, Value>
{
	pure fn mul(rhs: Unit) -> Value
	{
		apply_numer(self, rhs)
	}
}

// ---- Internal Items ------------------------------------------------------------------
pure fn apply_numer(value: Value, unit: Unit) -> Value
{
	match unit
	{
		// length
		Meter		=> Value {numer: value.numer + @[unit], ..value},
		Feet		=> Value {value: 0.3048*value.value, numer: value.numer + @[Meter], ..value},
		
		// time
		Second		=> Value {numer: value.numer + @[unit], ..value},
		Minute		=> Value {value: 60.0*value.value, numer: value.numer + @[Second], ..value},
		
		// modifiers
		Micro		=> Value {value: 1.0e-6*value.value, ..value},
		Milli		=> Value {value: 1.0e-3*value.value, ..value},
		Kilo		=> Value {value: 1.0e3*value.value, ..value},
	}
}

// This is used when building Values.
//struct Units
//{
//	let numer: @[Unit];
//	let denom: @[Unit];
//	let modifiers: @[Unit];
//}
//
//fn canonicalize(value: float, units: Units) -> Value
//{
//	Value {value: value, numer: units.numer, denom: units.denom} 
//}

//fn apply_modifier(value: float, modifer: Unit) -> float
//{
//}
//
//fn apply_numer(value: Value, unit: Unit) -> Value
//{
//}
//
//fn apply_denom(value: Value, unit: Unit) -> Value
//{
//}

// ---- Tests ---------------------------------------------------------------------------
#[cfg(test)]
fn check(actual: float, expected: float) -> bool
{
	if float::abs(actual - expected) > 0.001
	{
		io::stderr().write_line(fmt!("Found %f, but expected %f", actual, expected));
		return false;
	}
	return true;
}

#[test]
fn simple_mult()
{
	let x = Value(5.0);
	assert check(5.0, x.get());
	
	let x = Value(5.0)*Kilo;
	assert check(5000.0, x.get());
	
	let x = Value(5.0)*Feet;
	assert check(1.524, x.get());
	
	let x = Value(2.0)*Minute;
	assert check(120.0, x.get());
	
	let x = Value(5.0)*Kilo*Feet;
	assert check(1524.0, x.get());
}

// TODO:
// get needs to take units
// divide
// plus/minus
// maybe other ops
// stringify
//    probably need some sort of weight to order stuff like time last
// generic
// add a script to generate tables
//    use SI units
//    use IEC binary prefixes
//    use imperial units
// maybe turn this into a project
