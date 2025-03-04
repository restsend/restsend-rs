package com.restsend.restsend_sdk

import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4

import org.junit.Test
import org.junit.runner.RunWith

import org.junit.Assert.*
import uniffi.restsend_sdk.Client
import uniffi.restsend_sdk.guestLogin
import kotlinx.coroutines.runBlocking
/**
 * Instrumented test, which will execute on an Android device.
 *
 * See [testing documentation](http://d.android.com/tools/testing).
 */
@RunWith(AndroidJUnit4::class)
class ExampleInstrumentedTest {
    @Test
    fun connectChat() = runBlocking {
        var info = guestLogin("https://chat.ruzhila.cn", "android-test", null)
        var client = Client("","", info)
        client.connect()
    }    
}