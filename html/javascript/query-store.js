// Page that allows an arbitrary query to be run against a store.
"use strict";

function submit_query()
{
	if (GNOS.query_store)
	{
		GNOS.query_store.close();
		GNOS.query_store = null;
	}
	
	var name = document.getElementById('name');
	var query = document.getElementById('query');
	
	GNOS.query_store = new EventSource('/query?name={0}&expr={1}'.format(
		encodeURIComponent(name.value), encodeURIComponent(query.value)));
	GNOS.query_value = query.value;
	GNOS.query_store.addEventListener('message', function(event)
	{
		var data = JSON.parse(event.data);
		
		var element = document.getElementById('results');
		if (event.data[0] != '"')
			animated_draw(element, function() {save_solution(element, data);});
		else
			animated_draw(element, function() {save_err(element, data);});
	});
	
	GNOS.query_store.addEventListener('open', function(event)
	{
		console.log('> query-store stream opened');
	});
	
	GNOS.query_store.addEventListener('error', function(event)
	{
		if (event.eventPhase === 2)
		{
			console.log('> query-store stream closed');
		}
	});
}

function save_err(root, err)
{
	var html = "";
	html += "<section class='error'>";
	html += escapeHtml(err);
	html += "</section>";
	root.innerHTML = html;
}

function save_solution(root, solution)
{
	var headers = [];		// [header name]
	var rows = [];			// [[cell value (may be empty)]]
	analyze_solution(headers, rows, solution);
	
	var html = "";
	html += "<table border='1'>\n";
	html += "	<tr>\n";
	html += (headers.map(function (h) {return "<th>{0}</th>".format(escapeHtml(h || ""));})).join('\n');
	html += "	</tr>\n";
	for (var i = 0; i < rows.length; ++i)
	{
		var row = rows[i];
		
		html += "	<tr>\n";
		html += (row.map(function (r) {return "<td>{0}</td>".format(escapeHtml(r || ""));})).join('\n');
		html += "	</tr>\n";
	}
	html += "</table>\n";
	root.innerHTML = html;
}

// solution will be json of the form:
// [
//    {"name": "bob", "age": 10"}, 		row 0
//    ...										row 1
// ]
function analyze_solution(headers, rows, solution)
{
	var re = /SELECT((\s+\?\w+)+)/i;
	var matches = GNOS.query_value.match(re);
	if (matches)
	{
		// The json that we get back from the server uses a dict for each row so the results
		// are in some weird order based on hash values. To report the results in some sensible
		// order we grep the query string for the variables in the order the user wanted them.
		var selection = matches[1].trim();
		var variables = selection.split(/\s+/);
		var names = variables.map(function (v) {return v.slice(1);});
		names.forEach(function (n) {headers.push(n);});
		
		for (var i = 0; i < solution.length; ++i)
		{
			var srow = solution[i];
			var row = [];
			
			for (var key in srow)
			{
				var index = headers.indexOf(key);
				row[index] = srow[key];
			}
			
			rows.push(row);
		}
	}
}
