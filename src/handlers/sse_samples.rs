/// Uses Server Sent Events to push details of samples updates.
use oldcomm::{Chan, Port};
use std::map::{HashMap};
use samples::{Msg, RegisterMsg, DeregisterMsg, Detail};
use server = rwebserve;
use runits::units::*;
use runits::generated::*;

// Sends a list of json objects where each object is of the form: 
// {"sample_name": "eth1", "min": 1.0, "mean": 1.0, "max": 1.0, "units": "kbps"}.
pub fn sse_query(samples_chan: Chan<Msg>, request: &server::Request, push: server::PushChan) -> server::ControlChan
{
	let owner = copy request.params.get(@~"owner");
	
	do utils::spawn_moded_listener(task::ThreadPerCore) |control_port: server::ControlPort|
	{
		info!("starting %? samples stream", owner);
		let notify_port = Port();
		let notify_chan = Chan(&notify_port);
		
		let key = fmt!("query %?", ptr::addr_of(&notify_port));
		oldcomm::send(samples_chan, RegisterMsg(copy key, copy owner, notify_chan));
		
		let mut details = ~[];
		loop
		{
			match oldcomm::select2(notify_port, control_port)
			{
				either::Left(copy new_details) =>
				{
					details = do std::sort::merge_sort(new_details) |x, y| {x.sample_name <= y.sample_name};
					oldcomm::send(push, fmt!("retry: 5000\ndata: %s\n\n", details_to_json(details).to_str()));
				}
				either::Right(server::RefreshEvent) =>
				{
					oldcomm::send(push, fmt!("retry: 5000\ndata: %s\n\n", details_to_json(details).to_str()));
				}
				either::Right(server::CloseEvent) =>
				{
					info!("shutting down samples stream");
					oldcomm::send(samples_chan, DeregisterMsg(key));
					break;
				}
			}
		}
	}
}

priv fn details_to_json(details: &[Detail]) -> std::json::Json
{
	std::json::List(
		do vec::map(details) |detail|
		{
			detail_to_json(detail)
		})
}

priv fn detail_to_json(detail: &Detail) -> std::json::Json
{
	let value = from_units(detail.max, Kilo*Bit/Second);
	let value = value.normalize_si();
	
	let unit = from_units(1.0, Kilo*Bit/Second);
	let unit = unit.convert_to(value.units);
	
	let mut map = ~send_map::linear::LinearMap();
	map.insert(~"sample_name", std::json::String(copy detail.sample_name));
	map.insert(~"min", std::json::Number(detail.min*unit.value));
	map.insert(~"mean", std::json::Number(detail.mean*unit.value));
	map.insert(~"max", std::json::Number(detail.max*unit.value));
	map.insert(~"units", std::json::String(value.units.to_str()));
	
	std::json::Object(map)
}
