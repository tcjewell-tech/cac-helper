# Phase 1 hardware checklist (for Tom)

These are the Phase 1 checks that need you, your CAC, or a click on your screen. Steps 1 and 2 come first: **Phase 2 can't start until both are recorded** (ADR-007).

About 45 minutes in total. Do them in order. For each step, paste the output back into the Claude chat.

**Privacy.** Some outputs show your name and DoD ID number. The commands below are written to leave those out where possible. Where a step says so, replace your name and number with `XXXX` before pasting. Don't paste any of this into GitHub.

**Nothing here changes your Windows setup** except step 4 (installs OpenSC, which you can uninstall afterwards). Step 3 adds a throwaway test certificate to your Mac and removes it again.

---

## Step 1: Windows 11 + CAC (P1.5) — do this first

**Goal:** find out whether Windows 11 can use your CAC on its own, with no ActivClient.

Do this *before* installing any CAC software on that PC. If ActivClient, OpenSC or similar is already installed, still run it. Command A will show that.

Plug in the reader, insert the CAC, wait 10 seconds. Open **PowerShell** (Start menu, type `PowerShell`, press Enter). Run each block and paste the output.

**A. What Windows and what CAC software is on this PC**

```powershell
(Get-CimInstance Win32_OperatingSystem) | Format-List Caption, Version, OSArchitecture
Get-ItemProperty HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*, HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\* | Where-Object DisplayName -match "ActivClient|ActivID|OpenSC|SafeNet|90meter|InstallRoot" | Format-Table DisplayName, DisplayVersion
```

**B. Which certificates Windows sees on the card** (no name or ID number in this output)

```powershell
Get-ChildItem Cert:\CurrentUser\My | Where-Object Issuer -like "*U.S. Government*" | Format-List Issuer, NotAfter, HasPrivateKey, EnhancedKeyUsageList
```

**C. The smart card self-test.** It saves a file to your Desktop and shows only the lines without personal data.

```powershell
certutil -scinfo -silent > "$env:USERPROFILE\Desktop\cac-scinfo.txt" 2>&1
Select-String -Path "$env:USERPROFILE\Desktop\cac-scinfo.txt" -Pattern "^Reader:|Card:|Provider =|Issuer:|NotAfter:|Chain validates|Smart Card Logon|PIV|error|FAILED" | ForEach-Object Line
```

If C prints nothing, tell me. Keep the Desktop file for now in case I need one more line from it. Delete it when we're done.

**D. Try a real CAC login.** Use a DoD site you normally sign in to with your CAC. Also try "Login with DoD CAC" on <https://www.cyber.mil/>. Do it once in **Edge** and once in **Firefox**. For each browser, tell me:

1. Which site.
2. Did a "choose a certificate" box appear? How many certificates did it list?
3. Did the login work? If not, what was the error text?
4. Did the site show a "not secure" or certificate warning before the login? (That means the DoD root certificates aren't installed yet. If so, run DISA InstallRoot from <https://www.cyber.mil/pki-pke/tools-configuration-files/>, try again, and tell me you did.)

**E. If you ran InstallRoot** (now or at any time on this PC), tell me how it went. This matters: a test run of InstallRoot 5.6 on a blank Windows machine refused to install anything.

1. Which download did you use (64-bit, or "Non-Administrator")?
2. Did it open and install the certificates without an error? If there was an error, what did it say? A photo of the screen is fine.
3. Then run this and paste the output. It lists DoD certificates by store, with no personal data:

```powershell
foreach ($s in 'LocalMachine\Root','LocalMachine\CA','LocalMachine\Disallowed','CurrentUser\Root','CurrentUser\CA','CurrentUser\Disallowed') { Get-ChildItem "Cert:\$s" | Where-Object { $_.Subject -match 'DoD|U.S. Government' -or $_.Issuer -match 'DoD|U.S. Government' } | ForEach-Object { "{0} | {1} | {2} | issued by {3}" -f $s, $_.Thumbprint, ($_.Subject -replace ',.*'), ($_.Issuer -replace ',.*') } }
```

---

## Step 2: Mac + CAC (P1.6) — do this first too

**Goal:** the same question for macOS.

Plug in the reader, insert the CAC. Open **Terminal** (Cmd+Space, type `Terminal`). Run each line and paste the output.

```bash
sw_vers
```

```bash
security list-smartcards
```

```bash
sc_auth identities
```

```bash
system_profiler SPSmartCardsDataType
```

The last two show your name and DoD ID number. Replace them with `XXXX` before pasting.

Then try the CAC login from step 1 D in **Safari**, **Chrome** and **Firefox**, and answer the same four questions for each. On a Mac there is no InstallRoot. If you get a certificate warning, just tell me and stop there.

---

## Step 3: Mac trust prompt (P1.4) — no CAC needed

**Goal:** confirm that macOS lets a normal user trust a certificate after showing its password / Touch ID box. The app's Mac installer depends on this.

This makes a throwaway test certificate that expires in 2 days, trusts it, checks it, and removes it.

In Terminal, one line at a time:

```bash
cd ~/Desktop
```

```bash
openssl req -x509 -newkey rsa:2048 -nodes -keyout cachelper-test.key -out cachelper-test.cer -days 2 -subj "/CN=CAC Helper Throwaway Test CA"
```

```bash
security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain cachelper-test.cer; echo "exit code: $?"
```

A password or Touch ID box should appear here. Approve it. If you can, take a screenshot of the box first (Cmd+Shift+4).

```bash
security dump-trust-settings -d | grep -A 4 "CAC Helper Throwaway"
```

Now clean up. The `sudo` line asks for your Mac password in the Terminal.

```bash
security remove-trusted-cert -d cachelper-test.cer; echo "exit code: $?"
```

```bash
sudo security delete-certificate -c "CAC Helper Throwaway Test CA" /Library/Keychains/System.keychain
```

```bash
rm cachelper-test.key cachelper-test.cer
```

**Paste back:** everything the Terminal printed, and tell me whether the box appeared on the "add" line and on the "remove" line.

---

## Step 4: Does the CAC answer the legacy applet IDs? (P1.10) — after step 1

**Goal:** check that your card answers the card-detection codes the app plans to use. It also confirms where OpenSC installs itself (P1.2).

This step installs OpenSC, which is free CAC software. That is why it comes **after** step 1.

1. On the Windows 11 PC, download the latest 64-bit `.msi` from <https://github.com/OpenSC/OpenSC/releases> and install it with the default options.
2. Insert the CAC. In PowerShell:

```powershell
Test-Path "C:\Program Files\OpenSC Project\OpenSC\pkcs11\opensc-pkcs11.dll"
```

```powershell
& "C:\Program Files\OpenSC Project\OpenSC\tools\opensc-tool.exe" --list-readers
```

```powershell
& "C:\Program Files\OpenSC Project\OpenSC\tools\opensc-tool.exe" --atr --name
```

```powershell
& "C:\Program Files\OpenSC Project\OpenSC\tools\opensc-tool.exe" -s 00:A4:04:00:05:A0:00:00:03:08 -s 00:A4:04:00:07:A0:00:00:01:16:DB:00 -s 00:A4:04:00:07:A0:00:00:00:79:10:00 -s 00:A4:04:00:07:A0:00:00:00:79:01:00
```

**Paste back:** all of the output. It contains no personal data. Also tell me which OpenSC version you installed.

If `--list-readers` shows more than one reader, tell me before running the last two commands.

You can uninstall OpenSC afterwards (Settings → Apps → Installed apps).

---

## Step 5: Cancel a Windows admin prompt (P1.3) — any Windows PC, no CAC

**Goal:** see what Windows reports when someone says "No" to the admin prompt.

The command asks for admin rights to run a command that does nothing (`exit 0`).

Open **Command Prompt** (Start menu, type `cmd`). This one needs Command Prompt, not PowerShell. Paste this line:

```bat
powershell -NoProfile -NonInteractive -Command "$p = Start-Process cmd -ArgumentList @('/c','exit 0') -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode"
```

When the "Do you want to allow this app to make changes" box appears, click **No**. Then run:

```bat
echo %ERRORLEVEL%
```

**Paste back:** everything the window printed, including any red error text and the number from the second line.

---

## Not on your list

- **InstallRoot before/after comparison (P1.7).** It needs a throwaway Windows virtual machine, because InstallRoot changes the system certificate stores. Claude can't do it safely on the dev PC. If you already have a Windows VM, say so and I'll write the commands.
