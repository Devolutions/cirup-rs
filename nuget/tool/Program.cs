using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

return Run(args);

static int Run(string[] args)
{
    string? supportedRid = GetSupportedRid();
    string executableName = RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "cirup.exe" : "cirup";
    string? nativeExecutablePath = ResolveNativeExecutablePath(supportedRid, executableName);

    if (nativeExecutablePath is null)
    {
        PrintUnsupportedRidMessage(supportedRid);
        return 1;
    }

    EnsureExecutableBit(nativeExecutablePath);

    var processStartInfo = new ProcessStartInfo(nativeExecutablePath)
    {
        UseShellExecute = false,
    };

    foreach (string argument in args)
    {
        processStartInfo.ArgumentList.Add(argument);
    }

    using Process? process = Process.Start(processStartInfo);
    if (process is null)
    {
        Console.Error.WriteLine("Unable to start native cirup executable.");
        return 1;
    }

    process.WaitForExit();
    return process.ExitCode;
}

static string? GetSupportedRid()
{
    Architecture architecture = RuntimeInformation.ProcessArchitecture;

    if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
    {
        return architecture switch
        {
            Architecture.X64 => "win-x64",
            Architecture.Arm64 => "win-arm64",
            _ => null,
        };
    }

    if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
    {
        return architecture switch
        {
            Architecture.X64 => "linux-x64",
            Architecture.Arm64 => "linux-arm64",
            _ => null,
        };
    }

    if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
    {
        return architecture switch
        {
            Architecture.X64 => "osx-x64",
            Architecture.Arm64 => "osx-arm64",
            _ => null,
        };
    }

    return null;
}

static string? ResolveNativeExecutablePath(string? supportedRid, string executableName)
{
    if (!string.IsNullOrWhiteSpace(supportedRid))
    {
        string packagedPath = Path.Combine(AppContext.BaseDirectory, "native", supportedRid, executableName);
        if (File.Exists(packagedPath))
        {
            return packagedPath;
        }
    }

    string legacyPath = Path.Combine(AppContext.BaseDirectory, executableName);
    if (File.Exists(legacyPath))
    {
        return legacyPath;
    }

    return null;
}

static void PrintUnsupportedRidMessage(string? supportedRid)
{
    Console.Error.WriteLine("No native cirup executable is available for this platform in this package.");
    Console.Error.WriteLine($"Detected runtime identifier: {RuntimeInformation.RuntimeIdentifier}");
    Console.Error.WriteLine($"Resolved supported runtime identifier: {supportedRid ?? "unsupported"}");
    Console.Error.WriteLine("Supported runtime identifiers: win-x64, win-arm64, linux-x64, linux-arm64, osx-x64, osx-arm64.");
    Console.Error.WriteLine("Install the tool on a supported platform, or download platform-specific binaries from cirup release assets.");
}

static void EnsureExecutableBit(string nativeExecutablePath)
{
    if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
    {
        return;
    }

    const string chmodPath = "/bin/chmod";
    if (!File.Exists(chmodPath))
    {
        return;
    }

    try
    {
        var chmodStartInfo = new ProcessStartInfo(chmodPath)
        {
            UseShellExecute = false,
        };
        chmodStartInfo.ArgumentList.Add("+x");
        chmodStartInfo.ArgumentList.Add(nativeExecutablePath);

        using Process? chmodProcess = Process.Start(chmodStartInfo);
        chmodProcess?.WaitForExit();
    }
    catch
    {
    }
}
