#!/bin/sh

# Exit if any commands return a non-zero exit code
set -e

install() {
	if [ "$(uname)" = "Darwin" ]; then
		if [ "$(uname)" = "arm64" ]; then
			wget 'https://github.com/Cyanistic/mpdtrackr/releases/latest/download/mpdtrackr-aarch64-apple-darwin' -O mpdtrackr
		else
			wget 'https://github.com/Cyanistic/mpdtrackr/releases/latest/download/mpdtrackr-x86_64-apple-darwin' -O mpdtrackr
		fi
	elif [ "$(uname)" = "Linux" ]; then
		wget 'https://github.com/Cyanistic/mpdtrackr/releases/latest/download/mpdtrackr-x86_64-unknown-linux-gnu' -O mpdtrackr
	else
		printf "Unknown OS\nInstall Linux binary anyways? y/N: "
		read input
		input=$(echo "$input" | tr '[:upper:]' '[:lower:]')
		if [ "$input" = "y" ]; then
			wget 'https://github.com/Cyanistic/mpdtrackr/releases/latest/download/mpdtrackr-x86_64-unknown-linux-gnu' -O mpdtrackr
		else
			exit
		fi
	fi

	chmod +x ./mpdtrackr

	if [ "$(id -u)" -ne 0 ]; then
		echo "Root permission is required to move mpdtrackr into /usr/bin" >&2
		sudo mv mpdtrackr /usr/bin/mpdtrackr
	else
		mv mpdtrackr /usr/bin/mpdtrackr
	fi

	echo "Successfully installed mpdtrackr" >&2

	if [ "$(uname)" = "Linux" ]; then
		printf "Install and enable mpdtrackr systemd service? (This makes mpdtrackr automatically start on boot)\n Y/n: "
		read input
		input=$(echo "$input" | tr '[:upper:]' '[:lower:]')
		if [ "$input" != "n" ]; then
			wget 'https://raw.githubusercontent.com/Cyanistic/mpdtrackr/master/service/mpdtrackr.service' -O mpdtrackr.service
			if [ "$(id -u)" -ne 0 ]; then
				echo "Root permission is required to move mpdtrackr.service into /usr/lib/systemd/user" >&2
				sudo mv mpdtrackr.service /usr/lib/systemd/user/mpdtrackr.service
			else
				mv mpdtrackr.service /usr/lib/systemd/user/mpdtrackr.service
			fi
			systemctl --user enable --now mpdtrackr.service
			echo "Successfully installed and enabled mpdtrackr.service" >&2
		fi
	fi
}

uninstall() {
	if [ "$(id -u)" -ne 0 ]; then
		echo "Root permission is required to uninstall mpdtrackr" >&2
		sudo rm /usr/bin/mpdtrackr
	else
		rm /usr/bin/mpdtrackr
	fi
	if [ "$(uname)" = "Linux" ]; then
		printf "Delete and disable mpdtrackr systemd service? Y/n: "
		read input
		input=$(echo "$input" | tr '[:upper:]' '[:lower:]')
		if [ "$input" != "n" ]; then
			if [ "$(id -u)" -ne 0 ]; then
				echo "Root permission is required to delete mpdtrackr.service" >&2
				sudo rm /usr/lib/systemd/user/mpdtrackr.service
			else
				rm /usr/lib/systemd/user/mpdtrackr.service
			fi
			systemctl --user disable --now mpdtrackr.service
			echo "Successfully deleted and disabled mpdtrackr.service" >&2
		fi
	fi
}

if [ "$1" != "uninstall" ]; then
	install
else
	uninstall
fi
