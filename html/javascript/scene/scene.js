// Mutable class used to manipulate, draw, and hit test a list of shape objects.
// Shapes explicitly added and removed are assumed to be statically positioned.
// Shapes added via merge_graph are positioned using the arbor library.
"use strict";

function Scene(context)
{
	// Might make sense to have a dialog allowing users to configure these settings,
	// but from my (limited) testing it's difficult to do very much useful with them.
	this.shapes = ['graph'];
	this.particles = arbor.ParticleSystem(
	{
		repulsion: 2*1000,	// the force repelling nodes from each other (1000)
		stiffness: 3*600,		// the rigidity of the edges (600)
		friction: 1*0.5,		// the amount of damping in the system (0.5) [need a lot of this when there are a bunch of edges]
		gravity: false,			// an additional force attracting nodes to the origin (false)
		fps: 30,					// frames per second (55)
		ft: 0.02,				// timestep to use for stepping the simulation (0.02)
		precision: 0.6			// accuracy vs. speed in force calculations (zero is fast but jittery, one is smooth but cpu-intensive) (0.6)
	});
	
	var renderer =
	{
		init: function () {},
		redraw: function () {this.draw(context);}
	};
}

function evaluate(predicate)
{
	var passed = true;
	
	if (predicate)
	{
		try
		{
			var selection = GNOS.selection || {'name': 'map'};
			var context = {'options': GNOS.options, 'selection': selection};
			passed = eval_predicate(context, predicate);
		}
		catch (e)
		{
			console.log("'{0}' failed to evaluate: {1}".format(predicate, e));
		}
	}
	
	return passed;
}

// Sets the pixel dimensions used when drawing nodes and edges.
// Padding is interpreted as in the CSS padding property.
Scene.prototype.set_screen_size = function (width, height, padding)
{
	this.particles.screenSize(width, height);
	this.particles.screenPadding(padding);
};

// Adds a shape or an an array of shapes to be rendered above existing shapes.
Scene.prototype.append = function (shape)
{
	if (jQuery.isArray(shape))
		this.shapes = this.shapes.concat(shape);
	else
		this.shapes.push(shape);
};

// Adds a shape or an an array of shapes to be rendered below existing shapes.
Scene.prototype.prepend = function (shapes)
{
	if (jQuery.isArray(shape))
		this.shapes = shape.concat(this.shapes);
	else
		this.shapes = [shape].concat(this.shapes);
};

// Remove all statically positioned shapes.
Scene.prototype.remove_statics = function ()
{
	this.shapes = ['graph'];
};

// Adds nodes/edges not in the existing graph and removes existing nodes/edges not in the graph argument.
// Graph should be an object with nodes and edges attributes where:
// nodes is a mapping from node names to shapes
// edges is a mapping from source node names to destinaton node names and shapes
// e.g. 
// {
//    nodes:
//     {
//        winterfell: shape1,
//        the_wall: shape2,
//        white_harbor: shape3
//    }, 
//    edges:
//   {
//        winterfell:
//       {
//           the_wall: {shape4}
//           white_harbor: {shape5}
//       }
//    }
// }
Scene.prototype.merge_graph = function (graph)
{
	this.particles.merge(graph);
};

Scene.prototype.draw = function (context)
{
	var self = this;
	
	$.each(this.shapes, function (i, shape)
	{
		// We could save and restore the context here, but it seems to work out better
		// if the code that changes settings is the code that reverts it (among other
		// things this works a lot better with composite shapes).
		//
		// Here we set some of the most important canvas properties to awful values
		// to ensure that shapes set the properties that they care about instead of 
		// assuming that they are still reasonable.
		context.strokeStyle = 'magenta';
		context.fillStyle = 'magenta';
		context.lineWidth = 10;
		
		if (shape === 'graph')
		{
			self.do_adjust_graph_positions(context);
			self.do_draw_graph(context);
		}
		else
		{
			if (evaluate(shape.predicate))
				shape.draw(context);
		}
		
		// Make sure thet the properties we set still have their awful values.
		// If not then the shape didn't restore the context.
		assert(context.strokeStyle === '#ff00ff' && context.fillStyle === '#ff00ff' && context.lineWidth === 10, shape + " didn't restore context");
	});
};

// Note that this will only return shapes which have a true clickable property.
Scene.prototype.hit_test = function (pt)
{
	// Iterate backwards so that the first shapes that respond to
	// clicks are the shapes that appear on top.
	for (var i = this.shapes.length - 1; i >= 0; --i)
	{
		var shape = this.shapes[i];
		if (shape.clickable && shape.hit_test(pt))
			return shape;
	}
	
	return null;
};

Scene.prototype.toString = function ()
{
	return "Scene with {0} shapes".format(this.shapes.length);
};

// For nodes we could simply translate the context coordinate system.
// But that won't work for edges because we need to use the perimeter and
// not the center to draw directed lines (and many shapes are not symmetric).
Scene.prototype.do_adjust_graph_positions = function (context)
{
	function update_position(nodes, shape)
	{
		var left = nodes[shape.from_node];
		var right = nodes[shape.to_node];
		if (left && right && (left[1] || right[1]))
		{
			var left_pt = left[0].rect.intersect_line(right[0].center);	// TODO: need to offset centers if there are multiple relations between the entities
			var right_pt = right[0].rect.intersect_line(left[0].center);
			if (left_pt != undefined && right_pt != undefined)
			{
				var line = new Line(left_pt, right_pt);
				
				if (shape.styles.indexOf("line-type:directed") >= 0)
				{
					line = line.shrink(0, 3);		// might want to add a style for the outdent
				}
				else if (shape.styles.indexOf("line-type:bidirectional") >= 0)
				{
					line = line.shrink(3, 3);
				}
				
				shape.set_line(line);
			}
			else
			{
				var line = new Line(Point.zero, Point.zero);
			}
			shape.set_line(line);
		}
	}
	
	var changed = false;
	var nodes = {};			// node name => [node shape, position changed]
	
	this.particles.eachNode(function (node, pt)
	{
		var center = new Point(pt.x, pt.y);
		
		if (node.data.center.distance_squared(center) >= 1)
		{
			node.data.set_center(context, center);
			nodes[node.data.name] = [node.data, true];
			changed = true;
		}
		else
		{
			nodes[node.data.name] = [node.data, false];
		}
	});
	
	if (changed)
	{
		this.particles.eachEdge(function (edge)
		{
			update_position(nodes, edge.data);
		});
		
		$.each(this.shapes, function (i, shape)
		{
			if (shape.from_node && shape.to_node)
				update_position(nodes, shape);
		});
	}
};

Scene.prototype.do_draw_graph = function (context)
{
	if (GNOS.options['none'])
	{
		this.particles.eachEdge(function (edge)
		{
			edge.data.draw(context);
		});
	}
	
	this.particles.eachNode(function (node)
	{
		if (evaluate(node.data.predicate))
			node.data.draw(context);
	});
};
