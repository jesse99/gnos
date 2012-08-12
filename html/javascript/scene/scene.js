// Mutable class used to manipulate, draw, and hit test a list of shape objects.
"use strict";

function Scene()
{
	this.shapes = [];
}

Scene.prototype.append = function (shape)
{
	this.shapes.push(shape);
}

Scene.prototype.find = function (predicate)
{
	for (var i = 0; i < this.shapes.length; ++i)
	{
		if (predicate(this.shapes[i]))
			return this.shapes[i];
	}
	
	return null;
}

Scene.prototype.remove_all = function ()
{
	this.shapes = [];
}

Scene.prototype.remove_if = function (predicate)
{
	this.shapes = this.shapes.filter(
		function(shape)
		{
			return !predicate(shape);
		});
}

Scene.prototype.draw = function (context)
{
	for (var i = 0; i < this.shapes.length; ++i)
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
		
		var shape = this.shapes[i];
		shape.draw(context);
		
		// Make sure thet the properties we set still have their awful values.
		// If not then the shape didn't restore the context.
		assert(context.strokeStyle === '#ff00ff' && context.fillStyle === '#ff00ff' && context.lineWidth === 10, shape + " didn't restore context");
	}
}

Scene.prototype.hit_test = function (pt)
{
	// Iterate backwards so that the first shapes that respond to
	// clicks are the shapes that appear on top.
	for (var i = this.shapes.length - 1; i >= 0; --i)
	{
		if (this.shapes[i].hit_test(pt))
			return this.shapes[i];
	}
	
	return null;
}

Scene.prototype.toString = function ()
{
	return "Scene with {0} shapes".format(this.shapes.length);
}
