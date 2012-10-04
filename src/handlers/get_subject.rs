/// Provides a view of the predicates owned by a particular subject. This is not 
/// intended to be something used very often: it's just a convenient mechanism 
/// by which users can inspect the raw data used by the other views.
use mustache::*;
use server = rwebserve::rwebserve;

fn get_subject(options: &options::Options, request: &server::Request, response: &server::Response) -> server::Response
{
	let name = request.matches.get(@~"name");
	let subject = request.matches.get(@~"subject");
	response.context.insert(@~"network-name", mustache::Str(@copy options.network_name));
	response.context.insert(@~"name", mustache::Str(name));
	response.context.insert(@~"subject", mustache::Str(subject));
	
	server::Response {template: ~"subject.html", ..*response}
}
