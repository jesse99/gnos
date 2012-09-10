use mustache::*;
use server = rwebserve::rwebserve;

// Provides a view onto the back-end model used by gnos. This is not intended
// to be something used very often: it's just a convenient mechanism by which
// users can inspect the raw data used by the other views.
fn get_subject(request: &server::Request, response: &server::Response) -> server::Response
{
	let name = request.matches.get(@~"name");
	let subject = request.matches.get(@~"subject");
	response.context.insert(@~"name", mustache::Str(name));
	response.context.insert(@~"subject", mustache::Str(subject));
	
	server::Response {template: ~"subject.html", ..*response}
}
