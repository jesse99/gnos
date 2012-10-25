/// Provides a view for the details associated with an entity.
use mustache::*;
use server = rwebserve::rwebserve;

pub fn get_details(options: &options::Options, request: &server::Request, response: &server::Response) -> server::Response
{
	let name = request.matches.get(@~"name");
	let subject = request.matches.get(@~"subject");
	response.context.insert(@~"network-name", Str(@copy options.network_name));
	response.context.insert(@~"name", Str(name));
	response.context.insert(@~"subject", Str(subject));
	
	let i = str::rfind_char(*subject, '/');
	response.context.insert(@~"label", Str(@subject.slice(i.get()+1, subject.len())));
	
	server::Response {template: ~"details.html", ..*response}
}
