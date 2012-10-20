/// Helpers to extract information from the JSON sent back by modelers.
use std::json::*;
use send_map::linear::*;
use std::map::*;
use rrdf::rrdf::*;
use runits::generated::*;
use runits::units::*;
use Option = option::Option;

pub struct Snmp
{
	priv data: LinearMap<~str, Json>,	// the data from a modeler, TODO: should use @~str, but that's problematic bacause json doesn't atm
	pub new_time: Value,					// time (measured on the remote device) for the current snapshot
	
	priv old: Solution,						// if non-empty then selected bits of information from the pervious data
	priv old_prefix: ~str,					// old predicate = old prefix +":" + data snmp key
	priv old_subject: Option<Object>,	// old subject to use 
	
	priv delta_time: Option<Value>,		// time between current and previous snapshot
}

pub fn Snmp(device: &LinearMap<~str, Json>, data: &LinearMap<~str, Json>, old: Solution, old_prefix: ~str, old_subject: Option<Object>) -> Snmp
{
	let new_time = do get_new_value(device, &~"sysUpTime", Centi*Second).chain_ref |u| {option::Some(u.convert_to(Second))};
	let old_time = get_old_value(&old_subject, ~"gnos:timestamp", &old, Second);
	let delta_time =
		if new_time.is_some() && old_time.is_some()
		{
			option::Some(new_time.get() - old_time.get())
		}
		else
		{
			option::None
		};
	
	Snmp
	{
		data: copy *data,
		new_time: new_time.get_default(from_units(0.0, Second)),
		old: old,
		old_prefix: old_prefix,
		old_subject: old_subject,
		delta_time: delta_time,
	}
}

pub impl &Snmp
{
	// It would be nice if these methods normalized their results but we stick the value
	// into a store so we need to ensure that the units are known.
	fn get_value(key: &~str, units: Unit) -> Option<Value>
	{
		do get_new_value(&self.data, key, units).chain_ref
		|new_value|
		{
			option::Some(*new_value)
		}
	}
	
	// Snmp key should be in units units. If there was a previous value then the result
	// is in out_units/Second. Otherwise it will be in out_units.
	fn get_value_per_sec(key: &~str, units: Unit) -> Option<Value>
	{
		do get_new_value(&self.data, key, units).chain_ref
		|new_value|
		{
			let name = self.old_prefix + *key;
			let old_value = get_old_value(&self.old_subject, name, &self.old, units);
			
			if old_value.is_some() && self.delta_time.is_some() && self.delta_time.get().value > 1.0
			{
				let ps = (new_value - old_value.get())/self.delta_time.get();
				option::Some(ps)
			}
			else
			{
				option::Some(*new_value)
			}
		}
	}
}

pub fn lookup(table: &LinearMap<~str, Json>, key: &~str, default: &str) -> ~str
{
	match table.find(key)
	{
		option::Some(std::json::String(ref s)) =>
		{
			s.to_unique()
		}
		option::Some(ref value) =>
		{
			// This is something that should never happen so it's not so bad that we don't provide a lot of context
			// (if it does somehow happen admins can crank up the logging level to see where it is coming from).
			error!("%s was expected to be a string but is a %?", *key, value);	// TODO: would be nice if the site could somehow show logs
			default.to_unique()
		}
		option::None =>
		{
			default.to_unique()
		}
	}
}

// ---- Internal Items ------------------------------------------------------------------
priv fn get_new_value(data: &LinearMap<~str, Json>, key: &~str, units: Unit) -> Option<Value>
{
	match lookup(data, key, ~"")
	{
		~"" =>
		{
			option::None
		}
		ref text =>
		{
			match float::from_str(*text)
			{
				option::Some(value) =>
				{
					option::Some(from_units(value, units))
				}
				option::None =>
				{
					error!("%s was %s, but expected an int", *key, *text);
					option::None
				}
			}
		}
	}
}

priv fn get_old_value(subject: &Option<Object>, predicate: &str, old: &Solution, units: Unit) -> Option<Value>
{
	let old_row = old.rows.find(|r| {r.search(~"subject") == *subject && r.search(~"name") == option::Some(StringValue(predicate.to_unique(), ~""))});
	if old_row.is_some()
	{
		let x = from_units(old_row.get().get(~"value").as_float(), units);
		option::Some(x)
	}
	else
	{
		option::None
	}
}
