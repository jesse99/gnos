#!/usr/bin/python
# This script uses snmp to periodically model a network, encodes it into a json 
# dictionary, and ships the dictionary off to gnos using an http POST.  This
# is designed to be a generic modeler suitable for pretty much any device 
# running SNMP. Other modelers can be used to model more specialized
# functionality (like OSPF and PIM).
#
# We use a Python script instead of simply doing this within gnos for a few
# different reasons:
# 1) There are already Python snmp wrapper libraries. (This was written when
# the code still used pysnmp. Unfortunately pysnmp is not documented very well
# once you start doing anything sophisticated. It also has a rather ridiculous API).
# 2) Using a separate script will make it easier for gnos to manage multiple LANs.
# 3) This separation simplifies development. In particular gnos can run on a 
# developer machine and the script can run on an arbitrary machine connected
# to an arbitrary LAN.
# 4) This design makes it easy for users to write custom modelers using ssh
# or whatever.
import json, socket

logger = None

def set_logger(x):
	global logger
	logger = x

def add_label(data, target, label, key, level = 0, style = ''):
	if label:
		sort_key = '%s-%s' % (level, key)
		if style:
			data['labels'].append({'target-id': target, 'label': label, 'level': level, 'sort-key': sort_key, 'style': style})
		else:
			data['labels'].append({'target-id': target, 'label': label, 'level': level, 'sort-key': sort_key})
		
def add_gauge(data, target, label, value, level, style, sort_key):
	data['gauges'].append({'entity-id': target, 'label': label, 'value': value, 'level': level, 'style': style, 'sort-key': sort_key})

def add_details(data, target, label, details, opened, sort_key, key):
	data['details'].append({'entity-id': target, 'label': label, 'details': json.dumps(details), 'open': opened, 'sort-key': sort_key, 'id': key})

def add_relation(data, left, right, style = '', left_label = None, middle_label = None, right_label = None):
	relation = {'left-entity-id': left, 'right-entity-id': right, 'style': style}
	if left_label:
		relation['left-label'] = left_label
	if middle_label:
		relation['middle-label'] = middle_label
	if right_label:
		relation['right-label'] = right_label
	data['relations'].append(relation)

def open_alert(data, target, key, mesg, resolution, kind):
	data['alerts'].append({'entity-id': target, 'key': key, 'mesg': mesg, 'resolution': resolution, 'kind': kind})

def close_alert(data, target, key):
	data['alerts'].append({'entity-id': target, 'key': key})

def secs_to_str(secs):
	if secs >= 365.25*86400:
		return '%.2f years' % (secs/(365.25*86400))		# http://en.wikipedia.org/wiki/Month#Month_lengths
	elif secs >= 365.25*86400/12:
		return '%.2f months' % (secs/(365.25*86400/12))
	elif secs >= 86400:
		return '%.1f days' % (secs/(86400))
	elif secs >= 60*60:
		return '%.1f hours' % (secs/(60*60))
	elif secs >= 60:
		return '%.0f minutes' % (secs/(60))
	elif secs >= 1:
		return '%.0f seconds' % secs
	else:
		return '%.3f msecs' % (1000*secs)
			
def get_subnet(s):
	def count_leading_ones(mask):
		count = 0
		
		bit = 1 << 31
		while bit > 0:
			if mask & bit == bit:
				count += 1
				bit >>= 1
			else:
				break
		
		return count
	
	def count_trailing_zeros(mask):
		count = 0
		
		bit = 1
		while bit < (1 << 32):
			if mask & bit == 0:
				count += 1
				bit <<= 1
			else:
				break
		
		return count;
	
	if s:
		parts = s.split('.')
		bytes = map(lambda p: int(p), parts)
		mask = reduce(lambda sum, current: 256*sum + current, bytes, 0)
		leading = count_leading_ones(mask)
		trailing = count_trailing_zeros(mask)
		if leading + trailing == 32:
			return leading
		else:
			return s		# unusual netmask where 0s and 1s are mixed.
	else:
		'?'

def send_update(config, connection, data):
	logger.debug("sending update")
	logger.debug("%s" % json.dumps(data, sort_keys = True, indent = 4))
	if connection:
		try:
			body = json.dumps(data)
			headers = {"Content-type": "application/json", "Accept": "text/html"}
			
			connection.request("PUT", config['path'], body, headers)
			response = connection.getresponse()
			response.read()			# we don't use this but we must call it (or, on the second call, we'll get ResponseNotReady errors)
			if not str(response.status).startswith('2'):
				logger.error("Error PUTing: %s %s" % (response.status, response.reason))
				raise Exception("PUT failed")
		except Exception as e:
			address = "%s:%s" % (config['server'], config['port'])
			logger.error("Error PUTing to %s:%s: %s" % (address, config['path'], e), exc_info = type(e) != socket.error)
			raise Exception("PUT failed")

def send_entities(config, connection):
	entities = []
	for (name, device) in config["devices"].items():
		style = "font-size:larger font-weight:bolder"
		entity = {"id": device['ip'], "label": name, "style": style}
		logger.debug("entity: %s" % entity)
		entities.append(entity)
	send_update(config, connection, {"modeler": "config", "entities": entities})
