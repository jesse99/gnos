# Misc functions that pretty much every Python modeler will need to use.
import json, logging, logging.handlers, socket, subprocess

logger = None

def configure_logging(args, file_name):
	global logger
	logger = logging.getLogger(file_name)
	if args.verbose <= 1:
		logger.setLevel(logging.WARNING)
	elif args.verbose == 2:
		logger.setLevel(logging.INFO)
	else:
		logger.setLevel(logging.DEBUG)
		
	if args.stdout:
		handler = logging.StreamHandler()
		formatter = logging.Formatter('%(asctime)s  %(message)s', datefmt = '%I:%M:%S')
	else:
		# Note that we don't use SysLogHandler because, on Ubuntu at least, /etc/default/syslogd
		# has to be configured to accept remote logging requests.
		handler = logging.FileHandler(file_name, mode = 'w')
		formatter = logging.Formatter('%(asctime)s  %(message)s', datefmt = '%m/%d %I:%M:%S %p')
	handler.setFormatter(formatter)
	logger.addHandler(handler)
	return logger

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

def add_relation(data, left, right, style = '', left_label = None, middle_label = None, right_label = None, predicate = None):
	relation = {'left-entity-id': left, 'right-entity-id': right, 'style': style}
	if left_label:
		relation['left-label'] = left_label
	if middle_label:
		relation['middle-label'] = middle_label
	if right_label:
		relation['right-label'] = right_label
	if predicate:
		relation['predicate'] = predicate
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

def run_process(command):
	process = subprocess.Popen(command, bufsize = 8*1024, shell = True, stdout = subprocess.PIPE, stderr = subprocess.PIPE)
	(outData, errData) = process.communicate()
	if process.returncode != 0:
		logger.error(errData)
		raise ValueError('return code was %s:' % process.returncode)
	return outData

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

# It's a little lame that the edges have to be specified in the network file (using
# the links list) but relations don't work so well as edges because there are
# often too many of them (which causes clutter and, even worse, causes a
# lot of instability in node positions when there are too many forces acting
# on nodes (even with very high friction levels)).
def send_entities(config, connection):
	def find_ip(config, name):
		for (candidate, device) in config["devices"].items():
			if candidate == name:
				return device['ip']
		logger.error("Couldn't find link to %s" % name)
		return ''
		
	entities = []
	relations = []
	for (name, device) in config["devices"].items():
		style = "font-size:larger font-weight:bolder"
		entity = {"id": device['ip'], "label": name, "style": style}
		logger.debug("entity: %s" % entity)
		entities.append(entity)
		
		if 'links' in device:
			for link in device['links']:
				left = 'entities:%s' % device['ip']
				right = 'entities:%s' % find_ip(config, link)
				relation = {'left-entity-id': left, 'right-entity-id': right, 'predicate': 'options.none'}
				relations.append(relation)
	send_update(config, connection, {"modeler": "config", "entities": entities, 'relations': relations})
