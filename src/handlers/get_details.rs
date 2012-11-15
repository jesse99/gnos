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
	response.context.insert(@~"label", mustache::Str(@subject.slice(i.get()+1, subject.len())));
	error!("sending response");
	
	server::Response {template: ~"details.html", ..*response}
}
