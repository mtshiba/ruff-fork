import os

# Check all functions
subprocess.Popen("true")
subprocess.call("true")
subprocess.check_call("true")
subprocess.check_output("true")
subprocess.run("true")
os.system("true")
os.popen("true")
os.popen2("true")
os.popen3("true")
os.popen4("true")
popen2.popen2("true")
popen2.popen3("true")
popen2.popen4("true")
popen2.Popen3("true")
popen2.Popen4("true")
commands.getoutput("true")
commands.getstatusoutput("true")
os.execl("true")
os.execle("true")
os.execlp("true")
os.execlpe("true")
os.execv("true")
os.execve("true")
os.execvp("true")
os.execvpe("true")
os.spawnl("true")
os.spawnle("true")
os.spawnlp("true")
os.spawnlpe("true")
os.spawnv("true")
os.spawnve("true")
os.spawnvp("true")
os.spawnvpe("true")
os.startfile("true")

# Check it does not fail for full paths
os.system("/bin/ls")
os.system("./bin/ls")
os.system(["/bin/ls"])
os.system(["/bin/ls", "/tmp"])
os.system(r"C:\\bin\ls")
