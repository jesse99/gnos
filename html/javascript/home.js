// Page that shows a map of entities and the relations between them
"use strict";

GNOS.scene = new Scene();
GNOS.timer_id = undefined;
GNOS.last_update = undefined;
GNOS.poll_interval = undefined;
//GNOS.selection_name = null;
//GNOS.opened = {};
GNOS.entity_detail= undefined;
//GNOS.relation_detail= undefined;
GNOS.loaded_entities = false;

window.onload = function()
{
	resize_canvas();
	window.onresize = resize_canvas;
	
	GNOS.entity_detail = document.getElementById('entity_detail');

	var model_names = ["poll_interval", "entities", "labels", "gauges"];
	GNOS.entity_detail.onchange = function () {do_model_changed(model_names, false);};
	register_renderer("map renderer", model_names, "map", map_renderer);
	
	register_primary_map_query();
	GNOS.timer_id = setInterval(update_time, 1000);
	
	set_loading_label();
};

function resize_canvas()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	size_to_window(context);
	if (!GNOS.loaded_entities)
		set_loading_label();
	map_renderer(map, GNOS.sse_model, []);
}

function set_loading_label()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	var shape = new TextLineShape(context, new Point(map.width/2, map.height/2), 'Loading...', ["font-size:xx-large", "font-size:larger", "font-size:larger"]);
	GNOS.scene.remove_all();
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
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?poll_interval ?last_update								\
WHERE 														\
{																\
	?map gnos:poll_interval ?poll_interval .					\
	OPTIONAL												\
	{															\
		?map gnos:last_update ?last_update .					\
	}															\
}',
'																\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?target ?title ?style ?predicate								\
WHERE 														\
{																\
	?target gnos:entity ?title .									\
	OPTIONAL												\
	{															\
		?target gnos:style ?style		 .						\
	}															\
	OPTIONAL												\
	{															\
		?target gnos:predicate ?predicate .						\
	}															\
}',
	'															\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?label ?target ?level ?priority ?style ?predicate			\
WHERE 														\
{																\
	?info gnos:label ?label .									\
	?info gnos:target ?target .									\
	?info gnos:level ?level .									\
	?info gnos:priority ?priority .								\
	OPTIONAL												\
	{															\
		?info gnos:style ?style .								\
	}															\
	OPTIONAL												\
	{															\
		?info gnos:predicate ?predicate .						\
	}															\
}',
	'															\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?value ?target ?title ?level ?priority ?style ?predicate		\
WHERE 														\
{																\
	?gauge gnos:gauge ?value .								\
	?gauge gnos:target ?target .								\
	?gauge gnos:title ?title .									\
	?gauge gnos:level ?level .									\
	?gauge gnos:priority ?priority .							\
	OPTIONAL												\
	{															\
		?gauge gnos:style ?style .								\
	}															\
	OPTIONAL												\
	{															\
		?gauge gnos:predicate ?predicate .					\
	}															\
}'];
	var model_names = ["poll_interval", "entities", "labels", "gauges"];
	var callbacks = [poll_interval_query, entities_query, labels_query, gauges_query];
	register_query("primary map query", model_names, "primary", queries, callbacks);
}

// solution rows have 
// required fields: poll_interval
// optional fields: last_update
function poll_interval_query(solution)
{
	assert(solution.length <= 1, "expected one row but found " + solution.length);
	
	if (solution.length == 1)
	{
		var row = solution[0];
		GNOS.last_update = new Date().getTime();
		GNOS.poll_interval = row.poll_interval;
		
		var shape = create_poll_interval_label(GNOS.last_update, GNOS.poll_interval);
		
		return {poll_interval: shape};
	}
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
	if (solution.length > 0)
		GNOS.loaded_entities = true;
	
	var entities = [];
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	for (var i = 0; i < solution.length; ++i)
	{
		var row = solution[i];
		
		var style = row.style || "";
		var styles = style.split(' ');
		var label = new TextLineShape(context, Point.zero, row.title, styles, 0);
		
		entities.push({target: row.target, title: label, styles: styles, predicate: row.predicate || ""});
	}
	
	return {entities: entities};
}

// solution rows have 
// required fields: label, target, level, priority
// optional fields: style, predicate
function labels_query(solution)
{
	var labels = [];
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	for (var i = 0; i < solution.length; ++i)
	{
		var row = solution[i];
		
		var style = row.style || "";
		var styles = style.split(' ');
		var label = new TextLineShape(context, Point.zero, row.label, styles, row.priority);
		
		labels.push({target: row.target, shape: label, level: row.level, predicate: row.predicate || ""});
	}
	
	return {labels: labels};
}

// solution rows have 
// required fields: value, target, title, level, priority
// optional fields: style, predicate
function gauges_query(solution)
{
	var gauges = [];
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	for (var i = 0; i < solution.length; ++i)
	{
		var row = solution[i];
		
		var style = row.style || "";
		var styles = style.split(' ');
		var gauge = new GaugeShape(context, Point.zero, row.value, row.title, styles, row.priority);
		
		gauges.push({target: row.target, shape: gauge, level: row.level, predicate: row.predicate || ""});
	}
	
	return {gauges: gauges};
}

function map_renderer(element, model, model_names)
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	if (GNOS.loaded_entities)
	{
		var max_entity = 0;
		
		GNOS.scene.remove_all();
		model.entities.forEach(
			function (entity, i)
			{
				var child_shapes = [];
				child_shapes.push(entity.title);	
				
				var max_width = entity.title.width;
				model.labels.forEach(
					function (label)
					{
						if (label.target === entity.target && label.level <= GNOS.entity_detail.value)
						{
							child_shapes.push(label.shape);
							max_width = Math.max(label.shape.width, max_width);
						}
						
						max_entity = Math.max(label.level, max_entity);
					});
				model.gauges.forEach(
					function (gauge)
					{
						if (gauge.target === entity.target && gauge.level <= GNOS.entity_detail.value)
						{
							gauge.shape.adjust_width(context, max_width);
							child_shapes.push(gauge.shape);
						}
						
						max_entity = Math.max(gauge.level, max_entity);
					});
				
				// Ensure that info shapes appear in the same order on each entity.
				child_shapes.sort(
					function (x, y)
					{
						if (x.priority < y.priority)
							return -1;
						else if (x.priority > y.priority)
							return 1;
						else
							return 0;
					});
				
				// Unfortunately we can't create this shape until after all the other sub-shapes are created.
				// So it's simplest just to create the shape here.
				var center = new Point(200 + 200*i, 50 + 200*i);
//				var center = new Point(50 + 50*i * context.canvas.width, 50 + 50*i * context.canvas.height);
				var shape = new EntityShape(context, entity.target, center, entity.styles, child_shapes);
				GNOS.scene.append(shape);
			});
			
		// This is the only place where we know all of the levels of the entity infos.
		// If the range has changed we update the slider accordingly. (It's a bit weird
		// that we also use the slider value here but we can't do better).
		GNOS.entity_detail.max = max_entity;
		GNOS.entity_detail.hidden = max_entity === 0;
		
		if (model.poll_interval)								// this must be the last shape (we dynamically swap new shapes in)
			GNOS.scene.append(model.poll_interval);
		else
			GNOS.scene.append(new NoOpShape());
	}
	
	GNOS.scene.draw(context);
}

// ---- EntityShape class -------------------------------------------------------
// Used to draw a device consisting of a RectShape and an array of arbitrary shapes.
function EntityShape(context, name, center, styles, shapes)
{
	var width = 14 + shapes.reduce(function(value, shape)
	{
		return Math.max(value, shape.width);
	}, 0);
	this.total_height = 8 + shapes.reduce(function(value, shape)
	{
		return value + shape.height;
	}, 0);
	
	styles = ['frame-back-color:linen'].concat(styles);
	
	this.rect = new RectShape(context, new Rect(center.x - width/2, center.y - this.total_height/2, width, this.total_height), styles);
	this.shapes = shapes;
	this.name = name;
	this.clickable = true;
	freezeProps(this);
}

EntityShape.prototype.draw = function (context)
{
//	if (GNOS.selection_name == this.name)
//		this.rect.extra_styles = ['selection'];
//	else
//		this.rect.extra_styles = [];
	this.rect.draw(context);
	
	var dx = this.rect.geometry.left + this.rect.width/2;
	var dy = this.rect.geometry.top + this.rect.height/2 - this.total_height/2 + 0.2*this.shapes[0].height;	// TODO: hopefully when text metrics work a bit better we can get rid of that last term (think we need leading)
	for (var i = 0; i < this.shapes.length; ++i)
	{
		context.save();
		
		var shape = this.shapes[i];
		context.translate(dx, dy + shape.height/2);
		
		shape.draw(context);
		
		dy += shape.height;
		context.restore();
	}
};

EntityShape.prototype.hit_test = function (pt)
{
	return this.rect.hit_test(pt);
};

EntityShape.prototype.toString = function ()
{
	return "EntityShape for " + this.name;
};
