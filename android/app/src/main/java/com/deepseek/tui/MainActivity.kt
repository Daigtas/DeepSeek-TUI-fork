package com.deepseek.tui

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.ui.Modifier
import com.deepseek.tui.ui.DeepSeekRoot
import com.deepseek.tui.ui.theme.DeepSeekTheme

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        setContent {
            DeepSeekTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    DeepSeekRoot()
                }
            }
        }
    }
}
