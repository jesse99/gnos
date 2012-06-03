// This is the entry point into gnos web sites. It's designed to provide a quick visual
// indication of the health of the network as well as convenient navigation to other
// parts of the web site.
fn get_home(options: options, state_chan: comm::chan<msg>, _settings: hashmap<str, str>, _request: server::request, response: server::response) -> server::response
{
	let port = comm::port::<[triple]>();
	let chan = comm::chan::<[triple]>(port);
	comm::send(state_chan, getter(chan));
	let state = comm::recv(port);
	
	let mut triples = [];
	for vec::each(state)
	{|triple|
		let map = std::map::str_hash();
		map.insert("subject", mustache::str(triple.subject.to_str()));
		map.insert("property", mustache::str(triple.property));
		map.insert("object", mustache::str(triple.object.to_str()));
		vec::push(triples, mustache::map(map));
	};
	
	response.context.insert("store", mustache::vec(triples));
	response.context.insert("admin", mustache::bool(options.admin));
	{template: "home.html" with response}
}
