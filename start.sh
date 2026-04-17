#!/bin/bash
/app/coride-api &
exec nginx -g "daemon off;"
