// Page that shows details for a particular subject.
"use strict";

GNOS.store = undefined;

$(document).ready(function()
{
	var table = $('#body');
	GNOS.store = table.attr("data-name");
	GNOS.label = table.attr("data-label");
	var about = table.attr("data-about");
console.log('store: {0}'.format(GNOS.store));
console.log('about: {0}'.format(about));
	
	var query = '							\
SELECT 									\
	?detail									\
WHERE 									\
{											\
	?subject gnos:target {0} 	. 			\
	?subject gnos:detail ?detail 	. 		\
}'.format(about.replace('/', ':'));

	register_query("details", ["details"], GNOS.store, [query], [details_query]);
	
	register_renderer("details", ["details"], "body", details_renderer);
});

function details_query(solution)
{
	var html = '';
	$.each(solution, function (i, row)
	{
		var data = JSON.parse(row.detail);
		if ('markdown' in data)
		{
			html += "<h2>{0}</h2>".format(escapeHtml(GNOS.label));
			html += "<p>" + markdown.toHTML(data.markdown) + "</p>";
		}
		else
		{
			console.log("bad detail: {0}".format(row));
		}
	});
	
	if (!html)
	{
		html += "<h2>" + GNOS.label + "</h2>";
		html += "<p>No details</p>";
	}
	
	return {details: html};
}

function details_renderer(element, model, model_names)
{
	$('#body').html(model.details);
}
