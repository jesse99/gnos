// Page that shows a map of entities and the relations between them
"use strict";

GNOS.scene = undefined;
GNOS.timer_id = undefined;
GNOS.last_update = undefined;
GNOS.poll_interval = undefined;
GNOS.entity_detail = undefined;
GNOS.relation_detail = undefined;
GNOS.selection = undefined;
GNOS.loaded_entities = false;
GNOS.screen_padding = 80;		// px
GNOS.windows = {};
GNOS.options = {};

$(document).ready(function()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	GNOS.scene = new Scene(context);
	GNOS.options['none'] = true;
	
	var dropdown = $('#options_dropdown');
	dropdown.change(options_changed);
	
	resize_canvas();
	window.onresize = resize_canvas;
	
	GNOS.entity_detail = document.getElementById('entity_detail');
	GNOS.relation_detail = document.getElementById('relation_detail');
	
	var model_names = ["globals", "entities", "labels", "gauges", "alerts", "relations"];
	GNOS.entity_detail.onchange = function () {do_model_changed(model_names, false);};
	GNOS.relation_detail.onchange = function () {do_model_changed(model_names, false);};
	register_renderer("map renderer", model_names, "map", map_renderer);
	
	register_primary_map_query();
	GNOS.timer_id = setInterval(update_time, 1000);
	
	set_loading_label();
	initEntityDragging();
	$('#map').dblclick(handle_dblclick);
});

function update_predicates(predicate)
{
	function get_options(predicate)
	{
		var options = [];
		
		try
		{
			var expr = parse_predicate(predicate);
			$.each(expr, function (i, term)
			{
				if ($.isPlainObject(term) && 'type' in term && term.type === 'member' && term.target === 'options')
					options.push(term.member);
			});
		}
		catch (e)
		{
			console.log("'{0}' failed to parse: {1}".format(predicate, e));
		}
		
		return options;
	}
	
	$.each(get_options(predicate), function (i, option)
	{
		if (!(option in GNOS.options))
		{
			var dropdown = $('#options_dropdown')[0];
			dropdown.add(new Option(option.replace('_', ' '), option));
			GNOS.options[option] = false;
		}
	});
}

function options_changed(e)
{
	$('#options_dropdown').children().each(function (i, option)
	{
		GNOS.options[option.value] = option.selected;
	});
	console.log("selected: {0:j}".format(GNOS.options));
}

function handle_dblclick(e)
{
	var pos = $('#map').offset();
	var mouseP = arbor.Point(e.pageX - pos.left, e.pageY - pos.top);
	var obj = GNOS.scene.particles.nearest(mouseP);
	
	if (obj && obj.node !== null)
	{
		var url = obj.node.name.replace("/map/", "/details/");
		var i = url.indexOf('/details/');
		if (i > 0)
			url = url.slice(i);
		if (url in GNOS.windows && !GNOS.windows[url].closed)
			GNOS.windows[url].focus();
		else
			GNOS.windows[url] = window.open(url, obj.node.name);
	}
}

function initEntityDragging()
{
	var dragged = null;
	
	// set up a handler object that will initially listen for mousedowns then
	// for moves and mouseups while dragging
	var handlers =
	{
		clicked: function (e)
		{
			var pos = $('#map').offset();
			var mouseP = arbor.Point(e.pageX - pos.left, e.pageY - pos.top);
			dragged = GNOS.scene.particles.nearest(mouseP);
			
			// only allow the click if it isn't too far away from the entity
			if  (dragged && dragged.node && dragged.distance > dragged.node.data.radius)
				dragged = null;
			
			var map = document.getElementById('map');
			var context = map.getContext('2d');
			if (dragged && dragged.node !== null)
			{
				// while we're dragging, don't let physics move the node
				dragged.node.fixed = true;
				if (GNOS.selection !== dragged.node.data)
				{
					if (GNOS.selection)
						GNOS.selection.deselect(context);
					dragged.node.data.select(context);
				}
			}
			else if (GNOS.selection)
			{
				GNOS.selection.deselect(context);
			}
			
			$('#map').bind('mousemove', handlers.dragged);
			$(window).bind('mouseup', handlers.dropped);
			
			return false;
		},
		dragged: function (e)
		{
			var pos = $('#map').offset();
			var s = arbor.Point(e.pageX - pos.left, e.pageY - pos.top);
			
			if (dragged && dragged.node !== null)
			{
				var p = GNOS.scene.particles.fromScreen(s);
				dragged.node.p = p;
			}
			
			return false;
		},
		dropped: function (e)
		{
			if (dragged === null || dragged.node === undefined) return false;
			if (dragged.node !== null) dragged.node.fixed = false;		// setting this to true doesn't seem to do anything
			dragged.node.tempMass = 1000;
			dragged.node.mass = 1.0e50;									// not sure that this does anything
			dragged = null;
			$('#map').unbind('mousemove', handlers.dragged);
			$(window).unbind('mouseup', handlers.dropped);
			return false;
		}
	};
	
	$('#map').mousedown(handlers.clicked);
}

function resize_canvas()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	size_to_window(context);
	if (!GNOS.loaded_entities)
		set_loading_label();
	map_renderer(map, GNOS.sse_model, []);
	GNOS.scene.set_screen_size(map.width, map.height, GNOS.screen_padding);
}

function set_loading_label()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	var shape = new TextLineShape(context, new Point(map.width/2, map.height/2), 'Loading...', ["font-size:xx-large", "font-size:larger", "font-size:larger"]);
	GNOS.scene.remove_statics();
	GNOS.scene.append(shape);
}

function update_time()
{
	if (GNOS.scene.shapes.length > 0 && GNOS.poll_interval)
	{
		var shape = create_poll_interval_label(GNOS.last_update, GNOS.poll_interval);
		GNOS.scene.shapes[GNOS.scene.shapes.length-1] = shape;
		
		var map = document.getElementById('map');
		var context = map.getContext('2d');
		context.clearRect(0, 0, map.width, map.height);
		GNOS.scene.draw(context);
	}
}

function register_primary_map_query()
{
	// It's rather awkward to have all these OPTIONAL clauses, but according
	// to the spec the entire OPTIONAL block must match in order to affect 
	// the solution.
	var queries = [	'											\
SELECT 														\
	?poll_interval ?last_update ?num_errors					\
WHERE 														\
{																\
	?map gnos:poll_interval ?poll_interval .					\
	OPTIONAL												\
	{															\
		?map gnos:last_update ?last_update .					\
	}															\
	OPTIONAL												\
	{															\
		?map gnos:num_errors ?num_errors .					\
	}															\
}',
'																\
SELECT 														\
	?target ?title ?style ?predicate								\
WHERE 														\
{																\
	?target gnos:entity ?title .									\
	OPTIONAL												\
	{															\
		?target gnos:style ?style .								\
	}															\
	OPTIONAL												\
	{															\
		?target gnos:predicate ?predicate .						\
	}															\
}',
	'															\
SELECT 														\
	?subject ?label ?target ?level ?sort_key ?style ?predicate	\
WHERE 														\
{																\
	?subject gnos:label ?label .								\
	?subject gnos:target ?target .								\
	?subject gnos:level ?level .								\
	?subject gnos:sort_key ?sort_key .						\
	OPTIONAL												\
	{															\
		?subject gnos:style ?style .								\
	}															\
	OPTIONAL												\
	{															\
		?subject gnos:predicate ?predicate .					\
	}															\
}',
	'															\
SELECT 														\
	?value ?target ?title ?level ?sort_key ?style ?predicate		\
WHERE 														\
{																\
	?gauge gnos:gauge ?value .								\
	?gauge gnos:target ?target .								\
	?gauge gnos:title ?title .									\
	?gauge gnos:level ?level .									\
	?gauge gnos:sort_key ?sort_key .							\
	OPTIONAL												\
	{															\
		?gauge gnos:style ?style .								\
	}															\
	OPTIONAL												\
	{															\
		?gauge gnos:predicate ?predicate .					\
	}															\
}',
	'															\
SELECT 														\
	?target ?style												\
WHERE 														\
{																\
	?subject gnos:alert ?alert .									\
	?subject gnos:target ?target .								\
	?subject gnos:style ?style .									\
	OPTIONAL												\
	{															\
		?subject gnos:end ?end .								\
	}															\
	FILTER (!BOUND (?end))								\
}',
	'															\
SELECT 														\
	?left ?right 													\
	?style ?predicate ?left_info ?middle_info ?right_info		\
WHERE 														\
{																\
	?subject gnos:left ?left .									\
	?subject gnos:right ?right .									\
	OPTIONAL												\
	{															\
		?subject gnos:style ?style .								\
	}															\
	OPTIONAL												\
	{															\
		?subject gnos:predicate ?predicate .					\
	}															\
	OPTIONAL												\
	{															\
		?subject gnos:left_info ?left_info .						\
	}															\
	OPTIONAL												\
	{															\
		?subject gnos:middle_info ?middle_info .				\
	}															\
	OPTIONAL												\
	{															\
		?subject gnos:right_info ?right_info .					\
	}															\
}'];
	var model_names = ["globals", "entities", "labels", "gauges", "alerts", "relations"];
	var callbacks = [globals_query, entities_query, labels_query, gauges_query, alerts_query, relations_query];
	register_query("primary map query", model_names, "primary", queries, callbacks);
}

// solution rows have 
// required fields: poll_interval
// optional fields: last_update, num_errors
function globals_query(solution)
{
	assert(solution.length <= 1, "expected one row but found " + solution.length);
	
	if (solution.length == 1)
	{
		var row = solution[0];
		GNOS.last_update = new Date().getTime();
		GNOS.poll_interval = row.poll_interval;
		var poll_interval = create_poll_interval_label(GNOS.last_update, GNOS.poll_interval);
		
		var num_errors = row.num_errors || 0;
		if (num_errors > 0)
			var error_count = create_globals_err_label(num_errors);
		
		return {globals: {poll_interval: poll_interval, error_count: error_count}};
	}
	else
	{
		return {};
	}
}

function create_globals_err_label(num_errors)
{
	if (num_errors == 1)
		var mesg = "1 error";
	else
		var mesg = num_errors + " errors";
		
	var styles = ['font-size:large', 'font-weight:bolder', 'font-color:red'];
		
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	return new TextLineShape(context, function (self)
		{
			return new Point(context.canvas.width/2, context.canvas.height - self.stats.height/2);
		}, mesg, styles, 0);
}

function create_poll_interval_label(last, poll_interval)
{
	function get_updated_label(last, poll_interval)
	{
		var current = new Date().getTime();
		var last_delta = interval_to_time(current - last);
		
		if (poll_interval)
		{
			var next = last + 1000*poll_interval;
			if (current <= next)
			{
				var next_delta = interval_to_time(next - current);	
				var label = "updated {0} ago (next due in {1})".format(last_delta, next_delta);
				var style = "";
			}
			else if (current < next + 60*1000)		// next will be when modeler starts grabbing new data so there will be a bit of a delay before it makes it all the way to the client
			{
				var label = "updated {0} ago (next is due)".format(last_delta);
				var style = "";
			}
			else
			{
				var next_delta = interval_to_time(current - next);	
				var label = "updated {0} ago (next was due {1} ago)".format(last_delta, next_delta);
				var style = " font-color:red font-weight:bolder";
			}
		}
		else
		{
			// No longer updating (server has gone down or lost connection).
			var label = "updated {0} ago (not connected)".format(last_delta);
			var style = " font-color:red font-weight:bolder";
		}
		
		return [label, style];
	}
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	var labels = get_updated_label(last, poll_interval);
	var styles = ('font-size:smaller font-size:smaller' + labels[1]).split(' ');
	
	var shape = new TextLineShape(context,
		function (self)
		{
			return new Point(context.canvas.width/2, self.stats.height/2);
		}, labels[0], styles, 0);
	
	return shape;
}

// solution rows have 
// required fields: target, title
// optional fields: style, predicate
function entities_query(solution)
{
	GNOS.old_selection = GNOS.selection;
	GNOS.selection = null;
	
	if (solution.length > 0)
		GNOS.loaded_entities = true;
	
	var entities = [];
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	$.each(solution, function (i, row)
	{
		if (row.predicate)
			update_predicates(row.predicate);
		
		var style = row.style || "";
		var styles = style.split(' ');
		var label = new TextLineShape(context, Point.zero, row.title, styles, 0);
		
		entities.push({target: row.target, title: label, styles: styles, predicate: row.predicate || ""});
	});
	
	return {entities: entities};
}

// solution rows have 
// required fields: subject, label, target, level, sort_key
// optional fields: style, predicate
function labels_query(solution)
{
	var labels = {};
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	$.each(solution, function (i, row)
	{
		if (row.predicate)
			update_predicates(row.predicate);
		
		var style = row.style || "";
		var styles = style.split(' ');
		var label = new TextLineShape(context, Point.zero, row.label, styles, row.sort_key, row.predicate);
		
		labels[row.subject] = {target: row.target, shape: label, level: row.level, predicate: row.predicate || ""};
	});
	
	return {labels: labels};
}

// solution rows have 
// required fields: value, target, title, level, sort_key
// optional fields: style, predicate
function gauges_query(solution)
{
	var gauges = [];
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	$.each(solution, function (i, row)
	{
		if (row.predicate)
			update_predicates(row.predicate);
		
		var style = row.style || "";
		var styles = style.split(' ');
		var gauge = new GaugeShape(context, Point.zero, row.value, row.title, styles, row.sort_key, row.predicate);
		
		gauges.push({target: row.target, shape: gauge, level: row.level, predicate: row.predicate || ""});
	});
	
	return {gauges: gauges};
}

// solution rows have 
// required fields: target, style
function alerts_query(solution)
{
	function update_counts(table, row, style)
	{
		if (row.style.indexOf(style) >= 0)
		{
			if (!(row.target in table))
				table[row.target] = 1;
			else
				table[row.target] += 1;
		}
	}
	
	function add_alert(alerts, table, suffix, styles, level)
	{
		$.each(table, function (target, count)
		{
			if (count == 1)
				var label = "1 {0} alert".format(suffix);
			else
				var label = "{0} {1} alerts".format(count, suffix);
				
			var label = new TextLineShape(context, Point.zero, label, styles, 999);
			alerts.push({target: target, shape: label, level: level, predicate: ""});
		});
	}
	
	var errors = {};
	var warnings = {};
	var infos = {};
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	$.each(solution, function (i, row)
	{
		update_counts(errors, row, 'alert-type:error');
		update_counts(warnings, row, 'alert-type:warning');
		update_counts(infos, row, 'alert-type:info');
	});
	
	var alerts = [];
	add_alert(alerts, errors, "error", ['font-color:red', 'font-weight:bolder'], 0);
	add_alert(alerts, warnings, "warning", ['font-color:orange'], 2);
	add_alert(alerts, infos, "info", ['font-color:blue'], 3);
	
	return {alerts: alerts};
}

// solution rows have 
// required fields: left, right
// optional fields: style, predicate, left_info, middle_info, right_info
function relations_query(solution)
{
	var relations = [];
	$.each(solution, function (i, row)
	{
		if (row.predicate)
			update_predicates(row.predicate);
		
		var style = row.style || "";
		var styles = style.split(' ');
		
		relations.push({left: row.left, right: row.right, left_label: row.left_info, middle_label: row.middle_info, right_label: row.right_info, styles: styles, predicate: row.predicate || ""});
	});
	
	return {relations: relations};
}

function map_renderer(element, model, model_names)
{
	function add_node_styles(shape, styles)
	{
		var map = document.getElementById('map');
		$.each(styles, function (k, style)
		{
			if (style)
			{
				var i = style.indexOf(':');
				assert(i > 0, "failed to find ':' in " + style);
				
				var name = style.slice(0, i);
				var value = style.slice(i+1);
				
				if (name == "node-mass")
				{
					shape.mass = parseFloat(value);
				}
				else if (name == "node-start-x")
				{
					shape.x = parseFloat(value)*map.width;
				}
				else if (name == "node-start-y")
				{
					shape.y = parseFloat(value)*map.height;
				}
			}
		});
	}
	
	function get_nodes(model)
	{
		var max_entity = 0;
		var nodes = {};
		
		// add entity shapes to the scene,
		$.each(model.entities, function (k, entity)
		{
			var child_shapes = [];
			
			// title
			child_shapes.push(entity.title);	
			
			// labels
			var max_width = entity.title.width;
			$.each(model.labels, function (name, label)
			{
				if (label.target === entity.target && label.level <= GNOS.entity_detail.value)
				{
					child_shapes.push(label.shape);
					max_width = Math.max(label.shape.width, max_width);
				}
				
				max_entity = Math.max(label.level, max_entity);
			});
			
			// gauges
			$.each(model.gauges, function (i, gauge)
			{
				if (gauge.target === entity.target && gauge.level <= GNOS.entity_detail.value)
				{
					gauge.shape.adjust_width(context, max_width);
					child_shapes.push(gauge.shape);
				}
				
				max_entity = Math.max(gauge.level, max_entity);
			});
				
			// alerts
			$.each(model.alerts, function (i, alert)
			{
				if (alert.target === entity.target && alert.level <= GNOS.entity_detail.value)
				{
					child_shapes.push(alert.shape);
					max_width = Math.max(alert.shape.width, max_width);
				}
				
				max_entity = Math.max(alert.level, max_entity);
			});
			
			// Ensure that info shapes appear in the same order on each entity.
			child_shapes.sort(
				function (x, y)
				{
					if (x.sort_key < y.sort_key)
						return -1;
					else if (x.sort_key > y.sort_key)
						return 1;
					else
						return 0;
				});
			
			// Unfortunately we can't create this shape until after all the other sub-shapes are created.
			// So it's simplest just to create the shape here.
			var shape = new EntityShape(context, entity.target, Point.zero, entity.styles, child_shapes, entity.predicate);
			add_node_styles(shape, entity.styles);
			nodes[entity.target] = shape;
			
			if (GNOS.old_selection && GNOS.old_selection.name === entity.target)
				shape.select(context);
		});
		
		// This is the only place where we know all of the levels of the entity infos.
		// If the range has changed we update the slider accordingly. (It's a bit weird
		// that we also use the slider value here but we can't do better).
		GNOS.entity_detail.max = max_entity;
		show(['#entity_detail', '#entity_detail_label'], max_entity !== 0);
		
		return nodes;
	}
	
	function get_edges(context, model)
	{
		function add_label(model, shape, line, name, p, max_relation)
		{
			if (name && name in model.labels)
			{
				var label = model.labels[name];
				max_relation = Math.max(label.level, max_relation);
				
				if (label.level <= GNOS.relation_detail.value)
				{
					var styles = ['frame-width:0', 'frame-back-color:white'];
					var children = [label.shape];
					var child = new EntityShape(context, "", Point.zero, styles, children, label.predicate);
					shape.add_shape(p, child);
				}
			}
			
			return max_relation;
		}
		
		var edges = {};
		var max_relation = 0;
		
		$.each(model.relations, function (i, relation)
		{
			var line = new Line(new Point(0, 0), new Point(1, 0));
			if (relation.predicate == 'options.none')
			{
				if (!(relation.left in edges))
					edges[relation.left] = {};
				
				var shape = new LineShape(context, line, []);
				edges[relation.left][relation.right] = shape;
			}
			else
			{
				var shape = new LineShape(context, line, relation.styles, null, null, relation.predicate);
				GNOS.scene.append(shape);
				
				max_relation = add_label(model, shape, line, relation.left_label, 0.1, max_relation);
				max_relation = add_label(model, shape, line, relation.middle_label, 0.5, max_relation);
				max_relation = add_label(model, shape, line, relation.right_label, 0.9, max_relation);
			}
			shape.from_node = relation.left;
			shape.to_node = relation.right;
		});
		
		GNOS.relation_detail.max = max_relation;
		show(['#relation_detail', '#relation_detail_label'], max_relation !== 0);
		
		return edges;
	}
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	if (GNOS.loaded_entities)
	{
		GNOS.scene.remove_statics();
		
		var nodes = get_nodes(model);
		var edges = get_edges(context, model);
		GNOS.scene.merge_graph({nodes: nodes, edges: edges});
		
		if (model.globals && model.globals.error_count)
			GNOS.scene.append(model.globals.error_count);
		
		if (model.globals && model.globals.poll_interval)				// this must be the last shape (we dynamically swap new shapes in)
			GNOS.scene.append(model.globals.poll_interval);
		else
			GNOS.scene.append(new NoOpShape());
	}
	
	GNOS.scene.draw(context);
}

// ---- EntityShape class -------------------------------------------------------
// Used to draw a device consisting of a RectShape and an array of arbitrary shapes.
function EntityShape(context, name, center, styles, shapes, predicate)
{
	this.width = 14 + shapes.reduce(function(value, shape)
	{
		return Math.max(value, shape.width);
	}, 0);
	this.total_height = 8 + shapes.reduce(function(value, shape)
	{
		return value + shape.height;
	}, 0);
	this.radius = Math.max(this.width/2, this.total_height/2);
	
	this.base_styles = ['frame-back-color:linen'].concat(styles);
	this.styles = this.base_styles.slice(0);
	
	this.shapes = shapes;
	this.name = name;
	this.predicate = predicate;
	this.clickable = true;
	this.set_center(context, center);
}

EntityShape.prototype.set_center = function (context, center)
{
	this.rect = new RectShape(context, new Rect(center.x - this.width/2, center.y - this.total_height/2, this.width, this.total_height), this.styles, this.predicate);
	this.center = center;
};

EntityShape.prototype.select = function (context)
{
	this.styles = this.base_styles.concat(['frame-color:blue', 'frame-width:4']);
	this.rect.set_styles(context, this.styles);
	GNOS.selection = this;
};

EntityShape.prototype.deselect = function (context)
{
	this.styles = this.base_styles.slice(0);
	this.rect.set_styles(context, this.styles);
	GNOS.selection = null;
};

EntityShape.prototype.draw = function (context)
{
	this.rect.draw(context);
	
	var dx = this.rect.geometry.left + this.rect.width/2;
	var dy = this.rect.geometry.top + this.rect.height/2 - this.total_height/2 + 0.2*this.shapes[0].height;	// TODO: hopefully when text metrics work a bit better we can get rid of that last term (think we need leading)
	$.each(this.shapes, function (i, shape)
	{
		context.save();
		
		context.translate(dx, dy + shape.height/2);
		shape.draw(context);
		
		dy += shape.height;
		context.restore();
	});
};

EntityShape.prototype.hit_test = function (pt)
{
	return this.rect.hit_test(pt);
};

EntityShape.prototype.toString = function ()
{
	return "EntityShape for " + this.name;
};
