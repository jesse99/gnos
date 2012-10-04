/// Displays details about either in or out bandwidth for a device.
use mustache::*;
use server = rwebserve::rwebserve;

fn get_interfaces(request: &server::Request, response: &server::Response) -> server::Response
{
	let ip = request.matches.get(@~"managed_ip");
	let direction = request.matches.get(@~"direction");
	response.context.insert(@~"ip", mustache::Str(ip));
	response.context.insert(@~"direction", mustache::Str(direction));
	response.context.insert(@~"title", mustache::Str(@fmt!("%s %s Bandwidth", *ip, utils::title_case(*direction))));
	
	server::Response {template: ~"interfaces.html", ..*response}
}
