# Global
Configuration for global settings that apply to both server and client services.
### **GLOBAL_DEBUG**

If the value is set to `true`, the service will show proper logs and won't handle any panics.
Setting the DEBUG mode:
```console
foo@bar:~$ trabas global-config --set-debug
```
Unsetting the DEBUG mode:
```console
foo@bar:~$ trabas global-config --unset-debug
```
### **GLOBAL_LOG_LIMIT**

The maximum number logs will be shown in the console.
```console
foo@bar:~$ trabas global-config --log-limit 5
```
*note: This option does not apply to debug enabled logs.
