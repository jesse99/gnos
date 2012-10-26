// Page that shows details for a particular subject.
"use strict";

GNOS.store = undefined;
GNOS.opened = {};

$(document).ready(function()
{
	var table = $('#body');
	GNOS.store = table.attr("data-name");
	GNOS.label = table.attr("data-label");
	var about = table.attr("data-about");
console.log('store: {0}'.format(GNOS.store));
console.log('about: {0}'.format(about));
	
	var query = '						\
SELECT 								\
	?detail								\
WHERE 								\
{										\
	?subject gnos:target {0} . 			\
	?subject gnos:detail ?detail . 		\
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
		html += detail_to_html(data);
	});
	
	if (!html)
		html = "<p>No details</p>";
	
	html = "<h2>{0}</h2>\n{1}".format(escapeHtml(GNOS.label), html);
	
	return {details: html};
}

function detail_to_html(data)
{
	var html = '';
	
	if ('markdown' in data)
	{
		html = "<p>" + markdown.toHTML(data.markdown) + "</p>";
	}
	else if ('accordion' in data)
	{
		html = accordion_to_html(data.accordion);
	}
	else if ('table' in data)
	{
		html = table_to_html(data.table);
	}
	else
	{
		console.log("bad detail: {0:j}".format(data));
	}
	
	return html;
}

// required: title (may be empty), open, key
// optional: markdown, detail
function accordion_to_html(accordion)
{
	var html = '';
	
	$.each(accordion, function (i, data)
	{
		assert('markdown' in data || 'detail' in data, "expected markdown or detail in {0:j}".format(data));
		
		if ('markdown' in data)
			var inner = markdown.toHTML(data.markdown);
		else
			var inner = detail_to_html(data.detail);
		
		if (data.open === "always")
		{
			html += inner;
		}
		else
		{
			var open = GNOS.opened[data.key] || data.open === "yes";
			GNOS.opened[data.key] = open;
			
			var handler = "GNOS.opened['{0}'] = !GNOS.opened['{0}']".format(data.key);
			if (open)
				html += '<details open="open" onclick = "{0}">\n'.format(handler);
			else
				html += '<details onclick = "{0}">\n'.format(handler);
				
			if (data.title)
				html += '<summary>{0}</summary>\n'.format(escapeHtml(data.title));
				
			html += '{0}\n'.format(inner);
			html += '</details>\n';
		}
	});
	
	return html;
}

// required: style, header, rows
function table_to_html(table)
{
	var html = '';
	
	html += '<table border="1">\n';
		html += '<tr>\n';
		$.each(table.header, function (i, cell)
		{
			html += '<th>{0}</th>\n'.format(escapeHtml(cell));
		});
		html += '</tr>\n';
		
		if (table.style == 'plain')		// tables can be very large so we assume that all the content is formatted in the same way
		{
			$.each(table.rows, function (i, row)
			{
				html += '<tr>\n';
				$.each(row, function (j, cell)
				{
					html += '<td>{0}</td>\n'.format(escapeHtml(cell));
				});
				html += '</tr>\n';
			});
		}
		else if (table.style == 'html')
		{
			$.each(table.rows, function (i, row)
			{
				html += '<tr>\n';
				$.each(row, function (j, cell)
				{
					html += '<td>{0}</td>\n'.format(cell);
				});
				html += '</tr>\n';
			});
		}
		else if (table.style == 'markdown')
		{
			$.each(table.rows, function (i, row)
			{
				html += '<tr>\n';
				$.each(row, function (j, cell)
				{
					html += '<td>{0}</td>\n'.format(markdown.toHTML(cell));
				});
				html += '</tr>\n';
			});
		}
		else
		{
			console.log("bad style: " + table.style);
		}
	html += '</table>\n';
	
	return html;
}

function details_renderer(element, model, model_names)
{
	$('#body').html(model.details);
}
