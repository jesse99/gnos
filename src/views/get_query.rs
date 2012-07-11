// Uses Server Sent Events to send solutions for a query after the model is updated.
import rrdf::*;

export get_query;

fn get_query(_state_chan: comm::chan<msg>, _request: server::request, response: server::response) -> server::response
{
	response.headers.insert("Cache-Control", "no-cache");
	response.headers.insert("Content-Type", "text/event-stream");
	
	// Initial response is an empty solution.
	{body: "id: 1\nretry: 5000\ndata: []\n" with response}
}

