/// Uses Server Sent Events to push details of samples updates.
use comm::{Chan, Port};
use std::map::{HashMap};
use samples::{Msg, RegisterMsg, DeregisterMsg, Detail};
use server = rwebserve::rwebserve;
use rwebserve::imap::ImmutableMap;

// Sends a list of json objects where each object is of the form: 
// {"sample_name": "eth1", "min": 1.0, "mean": 1.0, "max": 1.0, "units": "kbps"}.
fn sse_query(samples_chan: Chan<Msg>, request: &server::Request, push: server::PushChan) -> server::ControlChan
{
	let owner = copy *request.params.get(@~"owner");
	
	do task::spawn_listener
	|control_port: server::ControlPort|
	{
		info!("starting %? samples stream", owner);
		let notify_port = Port();
		let notify_chan = Chan(notify_port);
		
		let key = fmt!("query %?", ptr::addr_of(notify_port));
		comm::send(samples_chan, RegisterMsg(copy key, copy owner, notify_chan));
		
		let mut details = ~[];
		loop
		{
			match comm::select2(notify_port, control_port)
			{
				either::Left(copy new_details) =>
				{
					details = std::sort::merge_sort(|x, y| {x.sample_name <= y.sample_name}, new_details);
					comm::send(push, fmt!("retry: 5000\ndata: %s\n\n", details_to_json(details).to_str()));
				}
				either::Right(server::RefreshEvent) =>
				{
					comm::send(push, fmt!("retry: 5000\ndata: %s\n\n", details_to_json(details).to_str()));
				}
				either::Right(server::CloseEvent) =>
				{
					info!("shutting down samples stream");
					comm::send(samples_chan, DeregisterMsg(key));
					break;
				}
			}
		}
	}
}

priv fn details_to_json(details: &[Detail]) -> std::json::Json
{
	std::json::List(@
		do vec::map(details) |detail|
		{
			detail_to_json(&detail)
		})
}

priv fn detail_to_json(detail: &Detail) -> std::json::Json
{
	let map = HashMap();
	map.insert(~"sample_name", std::json::String(@copy detail.sample_name));
	map.insert(~"min", std::json::Num(detail.min));
	map.insert(~"mean", std::json::Num(detail.mean));
	map.insert(~"max", std::json::Num(detail.max));
	map.insert(~"units", std::json::String(@copy detail.units));
	
	std::json::Dict(map)
}
