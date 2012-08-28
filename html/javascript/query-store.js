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
	GNOS.query_store.addEventListener('message', function(event)
	{
		var data = JSON.parse(event.data);
		
		var element = document.getElementById('results');
		animated_draw(element, function() {populate_results(element, data);});
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

function populate_results(root, data)
{
	var headers = [];		// [header name]
	var rows = [];			// [[cell value (may be empty)]]
	analyze_solution(headers, rows, data);
	
	var html = "";
	html += "<table border='1'>\n";
	html += "	<tr>\n";
	html += (headers.map(function (h) {return "<th>{0}</th>".format(escapeHtml(h || ""))})).join('\n');
	html += "	</tr>\n";
	for (var i = 0; i < rows.length; ++i)
	{
		var row = rows[i];
		
		html += "	<tr>\n";
		html += (row.map(function (r) {return "<td>{0}</td>".format(escapeHtml(r || ""))})).join('\n');
		html += "	</tr>\n";
	}
	html += "</table>\n";
	console.log("html: {0}".format(html));
	root.innerHTML = html;
}

// solution will be json of the form:
// [
//    {"name": "bob", "age": 10"}, 		row 0
//    ...										row 1
// ]
function analyze_solution(headers, rows, solution)
{
	for (var i = 0; i < solution.length; ++i)
	{
		var srow = solution[i];
		var row = [];
		
		// This is a little tricky because solution rows may have optional entries.
		for (var key in srow)
		{
			var index = headers.indexOf(key);
			if (index < 0)
			{
				index = headers.length;
				headers.push(key);			// unfortunately the order will be whacko because the rows are dicts
			}
			
			row[index] = srow[key];
		}
		
		rows.push(row);
	}
}
