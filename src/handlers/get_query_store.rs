// This is the entry point into gnos web sites. 

fn get_query_store(request: server::request, response: server::response) -> server::response
{
	{template: ~"query-store.html" with response}
}
