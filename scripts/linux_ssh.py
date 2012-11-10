#!/usr/bin/python
# Modeler designed for use on Linux devices that are not running SNMP. It's
# designed to be fairly minimal (if you want detailed information then get
# SNMP running).
import cgi, json, itertools, httplib, re, subprocess, sys, threading, time
from base_modeler import *

connection = None

def find_index(line, needle):
	parts = line.split()
	for i in xrange(0, len(parts)):
		if parts[i].startswith(needle):
			return i
	return None

class UName(object):
	def command(self):
		return 'uname -a'
	
	# Linux auto-fat 2.6.32-33-server #70-Ubuntu SMP Thu Jul 7 22:28:30 UTC 2011 x86_64 GNU/Linux
	def process(self, data, text, query):
		env.logger.debug("uname: '%s'" % text)
		target = 'entities:%s' % query.admin_ip
		add_label(data, target, query.admin_ip, 'a', level = 1, style = 'font-size:small')
		add_details(data, target, 'OS', [text], opened = 'always', sort_key = 'alpha', key = 'uname')
		
class Uptime(object):
	def command(self):
		return 'uptime'
		
	up_expr1 = re.compile(r'up \s+ (\d+) \s+ (min|sec|day)', re.MULTILINE | re.VERBOSE)
	up_expr2 = re.compile(r'up \s+ (\d+ : \d+) ,', re.MULTILINE | re.VERBOSE)
	load_expr = re.compile(r'load\ average: \s+ ([\d.]+)', re.MULTILINE | re.VERBOSE)
	
	# 04:43:21 up 0 min, load average: 1.02, 0.27, 0.09
	# 19:08:55 up  9:39, load average: 0.00, 0.01, 0.04
	# 07:13:42 up 52 days, 16:04,  3 users,  load average: 0.00, 0.00, 0.05
	def process(self, data, text, query):
		env.logger.debug("uptime: '%s'" % text)
		target = 'entities:%s' % query.admin_ip
		
		match = re.search(Uptime.up_expr1, text)
		if match:
			# Add a label with the uptime.
			add_label(data, target, 'uptime: %s %s' % (match.group(1), match.group(2)), 'alpha', level = 2, style = 'font-size:small')
			
			# Add an alert if the device has only been up a short time. There is potentially a lot
			# of variation here so, for now, we'll just match what we need. TODO: Sucks to do
			# all this lame parsing. Not sure how to do better though. Maybe proc files?
			if match.group(2) == 'sec' or (match.group(2) == 'min' and int(match.group(1)) <= 1):
				# TODO: Can we add something helpful for resolution? Some log files to look at? A web site?
				open_alert(data, target, key = 'uptime', mesg = 'Device rebooted.', resolution = '', kind = 'error')
			else:
				close_alert(data, target, key = 'uptime')
			
		match = re.search(Uptime.up_expr2, text)
		if match:
			add_label(data, target, 'uptime: %s' % match.group(1), 'alpha', level = 2, style = 'font-size:small')
			close_alert(data, target, key = 'uptime')
			
		# The load average is an average of the number of processes forced to wait
		# for CPU over the last 1, 5, and 15 minutes. We'll record the average for the 
		# last minute so that we can compute processor load after we know how many
		# cores there are.
		match = re.search(Uptime.load_expr, text)
		if match:
			query.load_average = float(match.group(1))
		
class CpuInfo(object):
	# Note that this will count both CPUs and cores.
	def command(self):
		return 'cat /proc/cpuinfo | grep -E "[Pp]rocessor[^:alpha:]+:" | wc -l'
		
	# 1
	def process(self, data, text, query):
		env.logger.debug("cpuinfo: '%s'" % text)
		if text.isdigit():
			query.num_cores = int(text)
		
class Df(object):
	def command(self):
		return 'df -h'
		
	# Filesystem                Size      Used Available Use% Mounted on
	# /dev/root                 6.6M      6.6M         0 100% /rom
	# tmpfs                    30.5M     60.0K     30.5M   0% /tmp
	# tmpfs                   512.0K         0    512.0K   0% /dev
	# /dev/mtdblock3            7.3M    724.0K      6.5M  10% /overlay
	# overlayfs:/overlay        7.3M    724.0K      6.5M  10% /
	def process(self, data, text, query):
		lines = text.splitlines()
		env.logger.debug("df: '%s'" % lines)
		
		use_index = find_index(lines[0], "Use%")
		mount_index = find_index(lines[0], "Mount")
		if use_index and mount_index:
			target = 'entities:%s' % query.admin_ip
			for line in lines[1:]:
				self.__process_line(data, target, line, use_index, mount_index)
				
	def __process_line(self, data, target, line, use_index, mount_index):
		parts = line.split()
		if parts[mount_index] != '/rom':
			value = int(parts[use_index][:-1])/100.0
			level = None
			if value >= 0.90:
				level = 1
				style = 'gauge-bar-color:salmon'
			elif value >= 0.75:
				level = 2
				style = 'gauge-bar-color:darkorange'
			elif value >= 0.50:
				level = 3
				style = 'gauge-bar-color:skyblue'
			if level:
				add_gauge(data, target, parts[mount_index], value, level, style, sort_key = 'zzz')
		
class Netstat(object):
	def command(self):
		return 'netstat -rn'
		
	# Kernel IP routing table
	# Destination     Gateway         Genmask         Flags   MSS Window  irtt Iface
	# 10.103.0.0      0.0.0.0         255.255.255.0   U         0 0          0 eth0
	# 0.0.0.0         10.103.0.2      0.0.0.0         UG        0 0          0 eth0
	def process(self, data, text, query):
		lines = text.splitlines()
		env.logger.debug("netstat: '%s'" % lines)
		
		# TODO: snmp-modeler can now figure this out so it's not needed. But it would be nice
		# to add a details table for routing.
#		gateway_index = find_index(lines[1], "Gateway")
#		if gateway_index:
#			target = 'entities:%s' % query.admin_ip
#			for line in lines[2:]:
#				self.__process_line(data, target, line, gateway_index)
				
	# TODO: In general the gateway IP will not be the admin IP. Not sure what the
	# best way to handle this is. Maybe we could point to an alias subject whose value
	# is the actual gateway device subject.
	def __process_line(self, data, target, line, gateway_index):
		parts = line.split()
		if parts[gateway_index] != '0.0.0.0':
			right = 'entities:%s' % parts[gateway_index]
			style = 'line-type:directed'
			predicate = "options.routes selection.name 'map' == and"
			add_relation(data, target, right, style, middle_label = {'label': 'gateway', 'level': 1, 'style': 'font-size:small'}, predicate = predicate)
			
# TODO:
# add interface table, use: /usr/sbin/ip address show
# add interface stats, use: /usr/sbin/ip -s  link (netstat -i would be nicer, but not always available)
# add routing table

class DeviceRunner(object):
	def __init__(self, ip, ssh, handlers):
		self.ip = ip
		self.__ssh = ssh
		self.__handlers = handlers
		commands = map(lambda x: '%s' % x.command(), handlers)
		self.__command = '; echo \'^^^\'; '.join(commands)
		
	def run(self):
		self.results = None
		command = '%s%s "%s"' % (self.__ssh, self.ip, self.__command)
		env.logger.debug(command)
		try:
			result = run_process(command)
			parts = result.split('^^^')
			if len(parts) == len(self.__handlers):
				self.results = map(lambda x: x.strip(), parts)
			else:
				raise Exception("Expected %s results but found '%s'" % (len(self.__handlers), result))
		except:
			env.logger.error("Error executing `%s`" % command, exc_info = True)

class QueryDevice(object):
	def __init__(self, device):
		self.__handlers = [UName(), Uptime(), CpuInfo(), Df(), Netstat()]
		self.__device = device
		self.admin_ip = device['ip']
		self.load_average = None		# 1 min load average
		self.num_cores = None
	
	def run(self, data, num_updates):
		runner = DeviceRunner(self.admin_ip, self.__device['ssh'], self.__handlers)
		runner.run()
		
		self.__process(runner, data)
		self.__add_cpu_load_gauge(data)
		
	def __process(self, runner, data):
		target = 'entities:%s' % runner.ip
		close_alert(data, target, key = 'device down')
		if runner.results:
			assert len(runner.results) == len(self.__handlers)
			for i in xrange(0, len(runner.results)):
				self.__handlers[i].process(data, runner.results[i], self)
		else:
			open_alert(data, target, key = 'device down', mesg = 'Device is down.', resolution = 'Check the power cable, power it on if it is off, check the IP address, verify routing.', kind = 'error')
		
	def __add_cpu_load_gauge(self, data):
		if self.load_average != None and self.num_cores != None:
			value = self.load_average/self.num_cores
			target = 'entities:%s' % self.admin_ip
			level = None
			if value >= 0.90:
				level = 1
				style = 'gauge-bar-color:salmon'
			elif value >= 0.75:
				level = 2
				style = 'gauge-bar-color:darkorange'
			elif value >= 0.50:
				level = 3
				style = 'gauge-bar-color:skyblue'
			if level:
				add_gauge(data, target, 'processor load', value, level, style, sort_key = 'y')
