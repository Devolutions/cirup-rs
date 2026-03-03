using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

return Run(args);

static int Run(string[] args)
{
    string executableName = RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "cirup.exe" : "cirup";
    string nativeExecutablePath = Path.Combine(AppContext.BaseDirectory, executableName);

    if (!File.Exists(nativeExecutablePath))
    {
        PrintUnsupportedRidMessage();
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

    using Process process = Process.Start(processStartInfo);
    if (process is null)
    {
        Console.Error.WriteLine("Unable to start native cirup executable.");
        return 1;
    }

    process.WaitForExit();
    return process.ExitCode;
}

static void PrintUnsupportedRidMessage()
{
    Console.Error.WriteLine("No native cirup executable is available for this runtime identifier in this package.");
    Console.Error.WriteLine($"Detected runtime identifier: {RuntimeInformation.RuntimeIdentifier}");
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

        using Process chmodProcess = Process.Start(chmodStartInfo);
        chmodProcess?.WaitForExit();
    }
    catch
    {
    }
}
