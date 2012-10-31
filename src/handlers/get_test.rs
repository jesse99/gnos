/// Provides a view of the results of running javascript unit tests.
use server = rwebserve::rwebserve;

pub fn get_test(_request: &server::Request, response: &server::Response) -> server::Response
{
	server::Response {template: ~"test.html", ..*response}
}
