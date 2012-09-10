// This is the entry point into gnos web sites. 
use server = rwebserve::rwebserve;

fn get_query_store(_request: &server::Request, response: &server::Response) -> server::Response
{
	server::Response {template: ~"query-store.html", ..*response}
}
