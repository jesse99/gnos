// Provides a view onto the back-end model used by gnos. This is not intended
// to be something used very often: it's just a convenient mechanism by which
// users can inspect the raw data used by the other views..
import rrdf::object::object_methods;
import rrdf::solution::solution_row_methods;
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
	{template: "(private)/subjects.html" with response}
}

fn get_subject(state_chan: comm::chan<msg>, request: server::request, response: server::response) -> server::response
{
	let subject = request.matches.get("subject");
	let rows = get_state(state_chan, #fmt["
		PREFIX gnos: <http://www.gnos.org/2012/schema#>
		SELECT
			?predicate ?object
		WHERE
		{
			%s ?p ?object .
			BIND(rrdf:pname(?p) AS ?predicate)
		} ORDER BY ?predicate ?object", subject]);

	let mut predicates = [];
	
	for vec::eachi(rows)
	|index, row|
	{
		let predicate = row.get("predicate").as_str();
		let object = row.get("object");
		
		let map = std::map::str_hash();
		map.insert("row-class", mustache::str(if index & 1u == 0u {"even"} else {"odd"}));
		map.insert("predicate", mustache::str(predicate));
		subject_utils::object_to_context(object, map);
		vec::push(predicates, mustache::map(map));
	}
	
	response.context.insert("subject", mustache::str(subject));
	response.context.insert("predicates", mustache::vec(predicates));
	
	{template: "(private)/subject.html" with response}
}

