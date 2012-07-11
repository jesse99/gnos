// Provides a view onto the back-end model used by gnos. This is not intended
// to be something used very often: it's just a convenient mechanism by which
// users can inspect the raw data used by the other views.
import rrdf::*;
import mustache::to_mustache;

fn get_subjects(state_chan: comm::chan<msg>, _request: server::request, response: server::response) -> server::response
{
	let rows = get_state(state_chan, "
		PREFIX gnos: <http://www.gnos.org/2012/schema#>
		SELECT DISTINCT
			?name
		WHERE
		{
			?subject ?predicate ?object .
			BIND(rrdf:pname(?subject) AS ?name)
		} ORDER BY ?name");

	let mut subjects = [];
	
	for vec::eachi(rows)
	|index, row|
	{
		let name = row.get("name").as_str();
		
		let map = std::map::str_hash();
		map.insert("row-class", mustache::str(if index & 1u == 0u {"even"} else {"odd"}));
		map.insert("url", mustache::str(#fmt["/subject/%s", name]));
		map.insert("text", mustache::str(name));
		vec::push(subjects, mustache::map(map));
	}
	
	response.context.insert("subjects", mustache::vec(subjects));
	{template: "subjects.html" with response}
}
