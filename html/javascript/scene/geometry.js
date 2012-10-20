// Immutable Point, Size, Rect, etc classes.
// All coordinates are screen coordinates.
// These are immutable in that the properties they define are fixed, but new mutable
// properties can be added to any of these.
"use strict";

// ---- Point class -----------------------------------------------------------
function Point(x, y)
{
	this.x = x;
	this.y = y;
	freezeProps(this);
}

Point.zero = new Point(0, 0);

// Returns the distance between this and rhs.
Point.prototype.distance = function (rhs)
{
	var dx = this.x - rhs.x;
	var dy = this.y - rhs.y;
	return Math.sqrt(dx*dx + dy*dy);
};

Point.prototype.distance_squared = function (rhs)
{
	var dx = this.x - rhs.x;
	var dy = this.y - rhs.y;
	return dx*dx + dy*dy;
};

Point.prototype.toString = function ()
{
	return "{x: " + this.x + ", y: " + this.y + "}";
};

// ---- Size class -----------------------------------------------------------
function Size(width, height)
{
	this.width = width;
	this.height = height;
	freezeProps(this);
}

Size.zero = new Size(0, 0);

Size.prototype.toString = function ()
{
	return "{width: " + this.width + ", height: " + this.height + "}";
};

// ---- Rect class ------------------------------------------------------------
function Rect(/*topLeft, size   or   left, top, width, height*/)
{
	var args = arguments;
	if (args.length == 2)
	{
		this.topLeft = topLeft;
		this.size = size;
	}
	else
	{
		assert(args.length == 4, "Expected {0} length to be 2 or 4".format(args));
		this.topLeft = new Point(args[0], args[1]);
		this.size = new Size(args[2], args[3]);
	}
	
	this.left = this.topLeft.x;
	this.top = this.topLeft.y;
	this.width = this.size.width;
	this.height = this.size.height;
	
	freezeProps(this);
}

Rect.prototype.toString = function ()
{
	return "{left: " + this.topLeft.x  + ", top: " + this.topLeft.y + ", width: " + this.size.width + ", height: " + this.size.height + "}";
};

// ---- Line class ------------------------------------------------------------
function Line(from, to)
{
	this.from = from;
	this.to = to;
	freezeProps(this);
}

Line.prototype.normals = function ()
{
	var unit = this.normalize();
	return [new Point(-unit.y, unit.x), new Point(unit.y, -unit.x)];
};

// Returns the line as a unit vector.
Line.prototype.normalize = function ()
{
	var x = this.to.x - this.from.x;
	var y = this.to.y - this.from.y;
	var d = this.to.distance(this.from);
	return new Point(x/d, y/d);
};

// Returns a point on the line from this.from (0.0) to this.to (1.0).
Line.prototype.interpolate = function (p)
{
	assert(p >= 0.0, "position is negative");
	assert(p <= 1.0, "position is larger than 1.0");
	
	var dx = this.to.x - this.from.x;
	var dy = this.to.y - this.from.y;
	
	return new Point(this.from.x + p*dx, this.from.y + p*dy);
};

// Shrinks the line by from_delta pixels on the from side and to_delta pixels on the to side.
Line.prototype.shrink = function (from_delta, to_delta)
{
	var length = this.to.distance(this.from);
	
	var from = this.relative_pt(from_delta/length);
	var to = this.relative_pt(1.0 - to_delta/length);
	
	return new Line(from, to);
};

// Returns a point along the line where p = 0.0 is from
// and p = 1.0 is to.
Line.prototype.relative_pt = function (p)
{
	return new Point(this.from.x + p*(this.to.x - this.from.x), this.from.y + p*(this.to.y - this.from.y));
};

// Returns either a Point where the two line segments intersect or null.
Line.prototype.intersection = function (other)
{
	// Based on the LeMothe code from http://stackoverflow.com/questions/563198/how-do-you-detect-where-two-line-segments-intersect
	var s1_x = this.to.x - this.from.x;
	var s1_y = this.to.y - this.from.y;
	var s2_x = other.to.x - other.from.x;
	var s2_y = other.to.y - other.from.y;
	
	var s = (-s1_y * (this.from.x - other.from.x) + s1_x * (this.from.y - other.from.y)) / (-s2_x * s1_y + s1_x * s2_y);
	var t = ( s2_x * (this.from.y - other.from.y) - s2_y * (this.from.x - other.from.x)) / (-s2_x * s1_y + s1_x * s2_y);
	
	if (s >= 0 && s <= 1 && t >= 0 && t <= 1)
		return new Point(this.from.x + (t * s1_x), this.from.y + (t * s1_y));
	
	return null;
};

Line.prototype.toString = function ()
{
	return "{from: " + this.from + ", to: " + this.to + "}";
};

// ---- Disc class ------------------------------------------------------------
function Disc(center, radius)
{
	this.center = center;
	this.radius = radius;
	freezeProps(this);
}

// Returns true if this intersects the rhs disc.
Disc.prototype.intersects = function (rhs)
{
	return this.center.distance(rhs.center) <= this.radius || rhs.center.distance(this.center) <= rhs.radius;
};

// Returns true if this intersects the rhs Point.
Disc.prototype.intersects_pt = function (rhs)
{
	return this.center.distance(rhs) <= this.radius;
};

// Returns the point on this perimeter closest to pt.
Disc.prototype.intersection = function (pt)
{
	var dx = pt.x - this.center.x;
	var dy = pt.y - this.center.y;
	var theta = Math.atan(dy/dx);
	
	var x = this.center.x + this.radius * Math.cos(theta);
	var y = this.center.y + this.radius * Math.sin(theta);
	var pt1 = new Point(x, y);
	
	x = this.center.x - this.radius * Math.cos(theta);
	y = this.center.y - this.radius * Math.sin(theta);
	var pt2 = new Point(x, y);
	
	if (pt1.distance(pt) < pt2.distance(pt))
		return pt1;
	else
		return pt2;
};

Disc.prototype.toString = function ()
{
	return "{center: " + this.center + ", radius: " + this.radius + "}";
};
