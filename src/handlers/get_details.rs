/// Provides a view for the details associated with an entity.
use server = rwebserve::rwebserve;

pub fn get_details(options: &options::Options, request: &server::Request, response: &server::Response) -> server::Response
{
	let name = request.matches.get(@~"name");
	let subject = request.matches.get(@~"subject");
	response.context.insert(@~"network-name", mustache::Str(@copy options.network_name));
	response.context.insert(@~"name", mustache::Str(name));
	response.context.insert(@~"subject", mustache::Str(subject));
	
	// subject will be something like entities/10.103.0.2
	let i = str::rfind_char(*subject, '/');
	let mut label = subject.slice(i.get()+1, subject.len());
	let m = vec::position(options.devices, |d| {d.managed_ip == label});
	if m.is_some()
	{
		label = fmt!("%s %s", options.devices[m.get()].name, label);
	}
	response.context.insert(@~"label", mustache::Str(@label));
	error!("sending response");
	
	server::Response {template: ~"details.html", ..*response}
}