// This is the entry point into gnos web sites. It's designed to provide a quick visual
// indication of the health of the network as well as convenient navigation to other
// parts of the web site.

// TODO:
// Home	Issues		Model	Admin Shutdown
// 
// Alerts
// 
// Devices
// -------------------------------
// Home is a link to this page.
// Issues is a link to a page showing history of alerts and warnings (maybe issues for today/yesterday and disclosure widgets for N previous days).
//     Might want issues to include alerts, errors, and warnings.
// Model is a link to a page showing the triple store.
// Admin allows configuration (managed devices, alert thresholds, etc).
// Shutdown is a link that kills the server (only shows up if admin).
//
// If there are no alerts or recent warnings Alerts should be big, green, and say "No Alerts".
// Alerts should be red and bold links.
// Device alerts should be a link to the device page.
// New warnings should cause a "2 new warnings" alert with a link to the issues page. 
//
// Devices should contain sorted links to device pages.
// Links should be color coded based on alert and warning status.

// TODO:
// need a query like
//    select name and managed_ip
//    where subject.starts_with("gnos:device")
fn get_home(options: options, channel: comm::chan<msg>, _settings: hashmap<str, str>, _request: server::request, response: server::response) -> server::response
{
	let state = get_state(channel);
	
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
