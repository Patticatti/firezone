"use client";

import Link from "next/link";
import ActionLink from "@/components/ActionLink";
import Image from "next/image";
import ConsentPreferences from "@/components/ConsentPreferences";

import {
  LinkedInIcon,
  GitHubIcon,
  XIcon,
  AppleIcon,
  WindowsIcon,
  LinuxIcon,
  AndroidIcon,
} from "@/components/Icons";

export default function Footer() {
  return (
    <footer className="relative bg-white border-t">
      <div className="mx-auto w-full max-w-screen-xl p-4 py-6 lg:py-8">
        <div className="md:flex md:justify-between">
          <div className="flex justify-between md:w-1/2 w-full mb-6 md:mb-0">
            <div>
              <Link href="/">
                <Image
                  width={150}
                  height={150}
                  src="/images/logo-text.svg"
                  alt="Firezone Logo"
                />
              </Link>
            </div>
            <div>
              <Link href="https://www.ycombinator.com/companies/firezone">
                <Image
                  width={150}
                  height={150}
                  src="/images/yc-logo.svg"
                  alt="YC Logo"
                />
              </Link>
            </div>
          </div>
          <div className="grid grid-cols-2 gap-8 sm:gap-6 sm:grid-cols-3">
            <div>
              <h2 className="mb-6 text-sm font-semibold text-neutral-900 uppercase ">
                Company
              </h2>
              <ul className="text-neutral-900  font-medium">
                <li className="mb-4">
                  <Link href="/" className="hover:underline">
                    Home
                  </Link>
                </li>
                <li className="mb-4">
                  <Link href="/about" className="hover:underline">
                    About
                  </Link>
                </li>
                <li className="mb-4">
                  <Link href="/pricing" className="hover:underline">
                    Pricing
                  </Link>
                </li>
                <li className="mb-4">
                  <Link
                    href="https://github.com/orgs/firezone/projects/9"
                    className="hover:underline hover:text-neutral-900"
                  >
                    Roadmap
                  </Link>
                </li>
                <li className="mb-4">
                  <Link href="/blog" className="hover:underline">
                    Blog
                  </Link>
                </li>
                <li>
                  <Link
                    href="https://www.ycombinator.com/companies/firezone"
                    className="hover:underline"
                  >
                    Jobs
                  </Link>
                </li>
              </ul>
            </div>
            <div>
              <h2 className="mb-6 text-sm font-semibold text-neutral-900 uppercase ">
                Resources
              </h2>
              <ul className="text-neutral-900  font-medium">
                <li className="mb-4">
                  <Link
                    href="/kb"
                    className="hover:underline hover:text-neutral-900"
                  >
                    Docs
                  </Link>
                </li>
                <li className="mb-4">
                  <Link
                    href="/support"
                    className="hover:underline hover:text-neutral-900"
                  >
                    Support
                  </Link>
                </li>
                <li className="mb-4">
                  <Link
                    href="/changelog"
                    className="hover:underline hover:text-neutral-900"
                  >
                    Changelog
                  </Link>
                </li>
                <li className="mb-4">
                  <Link
                    href="https://trust.firezone.dev/"
                    className="hover:underline hover:text-neutral-900"
                  >
                    Trust Center
                  </Link>
                </li>
                <li className="mb-4">
                  <Link
                    href="/contact/sales"
                    className="hover:underline hover:text-neutral-900"
                  >
                    Sales
                  </Link>
                </li>
                <li>
                  <Link
                    href="/product/newsletter"
                    className="hover:underline hover:text-neutral-900"
                  >
                    Newsletter
                  </Link>
                </li>
              </ul>
            </div>
            <div>
              <h2 className="mb-6 text-sm font-semibold text-neutral-900 uppercase ">
                Community
              </h2>
              <ul className="text-neutral-900  font-medium">
                <li className="mb-4">
                  <Link
                    href="https://discourse.firez.one"
                    className="hover:underline hover:text-neutral-900"
                  >
                    Forums
                  </Link>
                </li>
                <li className="mb-4">
                  <Link
                    href="https://discord.gg/DY8gxpSgep"
                    className="hover:underline hover:text-neutral-900"
                  >
                    Discord
                  </Link>
                </li>
                <li className="mb-4">
                  <Link
                    href="https://github.com/firezone"
                    className="hover:underline hover:text-neutral-900"
                  >
                    GitHub
                  </Link>
                </li>
                <li className="mb-4">
                  <Link
                    href="https://x.com/firezonehq"
                    className="hover:underline hover:text-neutral-900"
                  >
                    X
                  </Link>
                </li>
                <li>
                  <Link
                    href="https://www.linkedin.com/company/firezonehq"
                    className="hover:underline hover:text-neutral-900"
                  >
                    LinkedIn
                  </Link>
                </li>
              </ul>
            </div>
          </div>
        </div>
        <div className="sm:flex sm:justify-between sm:items-center mt-4 sm:mt-8">
          <div className="text-xs">
            <p>WireGuard is a registered trademark of Jason A. Donenfeld.</p>
            <p>Firezone is a registered trademark of Firezone, Inc.</p>
          </div>
          <div className="mt-4 sm:mt-0">
            <ActionLink
              href="https://probe.sh"
              size="ml-1 -mr-1 w-5 h-5"
              className="text-sm text-neutral-800 hover:underline"
            >
              Test your WireGuard connection
            </ActionLink>
          </div>
        </div>
        <hr className="my-2 border-neutral-200 sm:mx-auto lg:mb-8 lg:mt-4" />
        <div className="flex grid sm:grid-cols-3">
          <div className="text-xs text-neutral-900">
            © 2024{" "}
            <Link href="/" className="hover:underline">
              Firezone, Inc.
            </Link>{" "}
            <br />
            <Link href="/privacy-policy" className="hover:underline">
              privacy
            </Link>
            {" | "}
            <Link href="/terms" className="hover:underline">
              terms
            </Link>
            {" | "}
            <ConsentPreferences />
            {" | "}
            <Link
              href="https://app.termly.io/notify/1aa082a3-aba1-4169-b69b-c1d1b42b7a48"
              className="hover:underline"
            >
              do not sell or share my personal information
            </Link>
          </div>
          <div className="flex p-2 items-center justify-center space-x-5">
            <AppleIcon size={5} href="/kb/user-guides/macos-client" />
            <WindowsIcon size={5} href="/kb/user-guides/windows-client" />
            <LinuxIcon size={5} href="/kb/user-guides/linux-gui-client" />
            <AndroidIcon size={5} href="/kb/user-guides/android-client" />
          </div>
          <div className="flex p-2 items-center justify-center sm:justify-end space-x-5">
            <Link
              target="_blank"
              href={new URL("https://firezone.statuspage.io")}
              className="hover:underline text-xs"
            >
              Platform status
            </Link>
            <XIcon url={new URL("https://x.com/firezonehq")} />
            <GitHubIcon url={new URL("https://github.com/firezone")} />
            <LinkedInIcon
              url={new URL("https://linkedin.com/company/firezonehq")}
            />
          </div>
        </div>
      </div>
    </footer>
  );
}
