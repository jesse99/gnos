import model::{msg, close_alert, update_msg};

// Provides a view onto the back-end model used by gnos. This is not intended
// to be something used very often: it's just a convenient mechanism by which
// users can inspect the raw data used by the other views.
fn get_subjects(response: server::response, state_chan: comm::chan<msg>) -> server::response
{
	comm::send(state_chan, update_msg(~"alerts", |store, _msg| {close_alert(store, ~"server", ~"content")}, ~""));

	// There isn't much for us to do here: all the heavy lifting is done on
	// the client via javascript that uses server-sent events to dynamically
	// update the view based on a SPARQL query.
	{template: ~"subjects.html" with response}
}
